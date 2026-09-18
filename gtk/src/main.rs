//! GTK4 front end for omafil, built on the same `omafil-core` backend as the
//! Tauri build. See `docs/gtk-port.md` for what is ported and what is not.
//!
//! Destructive operations are not wired yet: until the undo and recycle
//! surfaces are ported, this build cannot delete anything.

use gtk4::{gio, glib, prelude::*, subclass::prelude::*};
use omafil_core::{drives, file_icons, file_manager_service, icon_theme, launch, listing, omarchy, operations, paths, store, thumbnail, watcher};
use listing::{DirectoryEntry, DirectoryEntryType, EntrySort};
use std::{cell::{Cell, RefCell}, path::PathBuf, rc::Rc};

mod row {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    pub struct Inner {
        pub name: RefCell<String>,
        pub icon: RefCell<String>,
        pub path: RefCell<String>,
        pub size: RefCell<String>,
        pub modified: RefCell<String>,
        pub is_directory: Cell<bool>,
        /// Resolved once per entry; None until asked for, Some(None) when this
        /// file type has no thumbnail.
        pub thumbnail: RefCell<Option<Option<String>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Inner {
        const NAME: &'static str = "OmafilRow";
        type Type = super::Row;
    }

    impl ObjectImpl for Inner {}
}

glib::wrapper! {
    pub struct Row(ObjectSubclass<row::Inner>);
}

impl Row {
    fn new(entry: &DirectoryEntry) -> Self {
        let object: Self = glib::Object::new();
        let inner = object.imp();
        let is_directory = entry.entry_type == DirectoryEntryType::Directory;
        inner.name.replace(entry.name.clone());
        inner.icon.replace(
            if is_directory { file_icons::folder_icon_name(&entry.name) } else { file_icons::file_icon_name(&entry.name) }.to_owned(),
        );
        inner.path.replace(entry.path.clone());
        inner.is_directory.set(is_directory);
        inner.size.replace(if is_directory { String::new() } else { format_bytes(entry.size) });
        inner.modified.replace(entry.modified.map(format_modified).unwrap_or_default());
        object
    }

    fn path(&self) -> String {
        self.imp().path.borrow().clone()
    }

    fn is_directory(&self) -> bool {
        self.imp().is_directory.get()
    }
}

fn format_bytes(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 { format!("{size} B") } else { format!("{value:.1} {}", UNITS[unit]) }
}

fn format_modified(seconds: i64) -> String {
    glib::DateTime::from_unix_local(seconds)
        .and_then(|stamp| stamp.format("%Y-%m-%d %H:%M"))
        .map(|text| text.to_string())
        .unwrap_or_default()
}

/// Omarchy's palette as GTK CSS. The webview build does this with custom
/// properties; GTK takes named colours the same way.
fn apply_theme() {
    // The display's icon theme is a singleton and refuses set_theme_name; the
    // supported route is the settings property, which GTK applies to it.
    if let (Some(settings), Some(theme)) = (gtk4::Settings::default(), icon_theme::current_theme_name()) {
        settings.set_gtk_icon_theme_name(Some(&theme));
    }
    let Some(colors) = omarchy::read_theme() else { return };
    let pick = |key: &str, fallback: &str| colors.get(key).cloned().unwrap_or_else(|| fallback.to_owned());
    let css = format!(
        "window, listview, headerbar {{ background: {background}; color: {foreground}; }}
         columnview listview > row:selected {{ background: {accent}; }}
         .brand {{ font-weight: 700; padding: 2px 6px 8px 6px; }}
         .heading {{ font-size: 0.85em; opacity: 0.6; padding: 0 6px; }}
         .shortcut {{ padding: 4px 6px; background: transparent; border: 0; color: {foreground}; }}
         .shortcut--active {{ background: {accent}; }}
         .tab {{ padding: 3px 4px 3px 10px; border-radius: 6px; }}
         .tab--active {{ background: {accent}; }}
         .crumb {{ padding: 2px 6px; background: transparent; border: 0; color: {foreground}; }}
         .cell {{ padding: 4px 8px; }}",
        background = pick("background", "#101010"),
        foreground = pick("foreground", "#e8e8e8"),
        accent = pick("color4", "#3b6ea5"),
    );
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(&css);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

/// One tab: its own folder, history and list. The header is shared and always
/// reflects whichever pane is showing.
struct Pane {
    folder: RefCell<PathBuf>,
    history: RefCell<Vec<PathBuf>>,
    store: gio::ListStore,
    selection: gtk4::MultiSelection,
    tab: gtk4::Box,
    label: gtk4::Label,
    page: gtk4::Widget,
}

struct App {
    _watchers: RefCell<Vec<Box<dyn std::any::Any>>>,
    /// Sidebar entries by the path they open, so the current folder can be
    /// marked without rebuilding the list.
    shortcuts: RefCell<Vec<(PathBuf, gtk4::Button)>>,
    panes: RefCell<Vec<Rc<Pane>>>,
    active: Cell<usize>,
    stack: gtk4::Stack,
    tabs: gtk4::Box,
    crumbs: gtk4::Box,
    status: gtk4::Label,
    back: gtk4::Button,
}

impl App {
    fn pane(&self) -> Rc<Pane> {
        let panes = self.panes.borrow();
        Rc::clone(&panes[self.active.get().min(panes.len() - 1)])
    }

    fn selected_rows(&self) -> Vec<Row> {
        let selection = &self.pane().selection;
        let bitset = selection.selection();
        (0..bitset.size())
            .filter_map(|index| selection.item(bitset.nth(index as u32)))
            .filter_map(|item| item.downcast::<Row>().ok())
            .collect()
    }
}

fn navigate(app: &Rc<App>, target: PathBuf, remember: bool) {
    let pane = app.pane();
    let listed = listing::read_directory_listing_revealing(
        target.to_string_lossy().into_owned(),
        EntrySort::Name, false, false, 0, listing::MAX_PAGE_SIZE, "", &[],
    );
    let Ok(listing) = listed else {
        app.status.set_text(&format!("{} could not be read.", target.display()));
        return;
    };

    if remember {
        pane.history.borrow_mut().push(pane.folder.borrow().clone());
    }
    pane.folder.replace(target.clone());
    pane.store.remove_all();
    for entry in &listing.entries {
        pane.store.append(&Row::new(entry));
    }
    pane.label.set_text(&paths::display_name(&target));

    while let Some(child) = app.crumbs.first_child() {
        app.crumbs.remove(&child);
    }
    for crumb in listing::path_crumbs(&target, &paths::navigable_roots()) {
        let button = gtk4::Button::builder().label(&crumb.name).css_classes(["crumb"]).has_frame(false).build();
        let destination = PathBuf::from(&crumb.path);
        let navigating = Rc::clone(app);
        button.connect_clicked(move |_| navigate(&navigating, destination.clone(), true));
        app.crumbs.append(&button);
    }

    for (path, button) in app.shortcuts.borrow().iter() {
        if *path == target {
            button.add_css_class("shortcut--active");
        } else {
            button.remove_css_class("shortcut--active");
        }
    }
    app.back.set_sensitive(!pane.history.borrow().is_empty());
    app.status.set_text(&format!(
        "{} items{}",
        listing.total,
        if listing.has_more { ", showing the first page" } else { "" }
    ));
}

/// Builds a tab and its page. The pane is not shown until `activate_tab`.
fn new_pane(state: &Rc<App>, folder: PathBuf) -> Rc<Pane> {
    let store = gio::ListStore::new::<Row>();
    let selection = gtk4::MultiSelection::new(Some(store.clone()));
    let view = gtk4::ColumnView::new(Some(selection.clone()));
    view.append_column(&name_column());
    view.append_column(&text_column("Size", false, |row| row.imp().size.borrow().clone()));
    view.append_column(&text_column("Modified", false, |row| row.imp().modified.borrow().clone()));
    view.set_enable_rubberband(true);

    let navigating = Rc::clone(state);
    view.connect_activate(move |view, position| {
        let Some(row) = view.model().and_then(|model| model.item(position)).and_downcast::<Row>() else { return };
        if row.is_directory() {
            navigate(&navigating, PathBuf::from(row.path()), true);
        }
    });
    let counting = Rc::clone(state);
    selection.connect_selection_changed(move |selection, _, _| {
        let chosen = selection.selection().size();
        if chosen > 0 {
            counting.status.set_text(&format!("{chosen} selected"));
        }
    });
    attach_context_menu(state, &view);

    let label = gtk4::Label::builder().label(paths::display_name(&folder)).ellipsize(gtk4::pango::EllipsizeMode::End).max_width_chars(16).build();
    let close = gtk4::Button::builder().icon_name("window-close-symbolic").css_classes(["tab-close"]).has_frame(false).build();
    let tab = gtk4::Box::builder().orientation(gtk4::Orientation::Horizontal).spacing(4).css_classes(["tab"]).build();
    tab.append(&label);
    tab.append(&close);

    let pane = Rc::new(Pane {
        folder: RefCell::new(folder),
        history: RefCell::new(Vec::new()),
        store,
        selection,
        tab: tab.clone(),
        label,
        page: gtk4::ScrolledWindow::builder().vexpand(true).child(&view).build().upcast(),
    });

    let switching = Rc::clone(state);
    let switch_to = Rc::clone(&pane);
    let press = gtk4::GestureClick::new();
    press.connect_pressed(move |_, _, _, _| {
        if let Some(index) = switching.panes.borrow().iter().position(|other| Rc::ptr_eq(other, &switch_to)) {
            activate_tab(&switching, index);
        }
    });
    tab.add_controller(press);

    let closing = Rc::clone(state);
    let close_this = Rc::clone(&pane);
    close.connect_clicked(move |_| {
        if let Some(index) = closing.panes.borrow().iter().position(|other| Rc::ptr_eq(other, &close_this)) {
            close_tab(&closing, index);
        }
    });

    pane
}

fn add_tab(state: &Rc<App>, folder: PathBuf) {
    let pane = new_pane(state, folder.clone());
    state.stack.add_child(&pane.page);
    state.tabs.append(&pane.tab);
    state.panes.borrow_mut().push(Rc::clone(&pane));
    let index = state.panes.borrow().len() - 1;
    activate_tab(state, index);
    navigate(state, folder, false);
}

fn activate_tab(state: &Rc<App>, index: usize) {
    let Some(pane) = state.panes.borrow().get(index).map(Rc::clone) else { return };
    state.active.set(index);
    state.stack.set_visible_child(&pane.page);
    for (position, other) in state.panes.borrow().iter().enumerate() {
        if position == index {
            other.tab.add_css_class("tab--active");
        } else {
            other.tab.remove_css_class("tab--active");
        }
    }
    // The header belongs to whichever pane is showing.
    let folder = pane.folder.borrow().clone();
    navigate(state, folder, false);
}

/// Closing the last tab closes nothing: a window with no pane has no state to
/// show and no way back.
fn close_tab(state: &Rc<App>, index: usize) {
    if state.panes.borrow().len() < 2 {
        return;
    }
    let pane = state.panes.borrow_mut().remove(index);
    state.stack.remove(&pane.page);
    state.tabs.remove(&pane.tab);
    activate_tab(state, index.min(state.panes.borrow().len() - 1));
}

fn shortcut(state: &Rc<App>, label: &str, icon: &str, target: PathBuf) -> gtk4::Button {
    let content = gtk4::Box::builder().orientation(gtk4::Orientation::Horizontal).spacing(8).build();
    content.append(&gtk4::Image::from_icon_name(icon));
    content.append(&gtk4::Label::builder().label(label).xalign(0.0).build());
    let button = gtk4::Button::builder().child(&content).css_classes(["shortcut"]).has_frame(false).build();
    let navigating = Rc::clone(state);
    let destination = target.clone();
    button.connect_clicked(move |_| navigate(&navigating, destination.clone(), true));
    state.shortcuts.borrow_mut().push((target, button.clone()));
    button
}

fn heading(text: &str) -> gtk4::Label {
    gtk4::Label::builder().label(text).xalign(0.0).css_classes(["heading"]).margin_top(10).build()
}

/// The sidebar omafil already shows, on the same data: XDG user directories
/// through `user_dirs`, saved pins and tags through `store`, and removable
/// media through `drives`. Locations the user has disabled or removed do not
/// appear, which is `known_directory_path` failing rather than a check here.
fn build_sidebar(state: &Rc<App>) -> gtk4::Widget {
    const USER_DIRECTORIES: [(&str, &str); 6] = [
        ("desktop", "Desktop"), ("downloads", "Downloads"), ("documents", "Documents"),
        ("pictures", "Pictures"), ("videos", "Videos"), ("music", "Music"),
    ];

    let column = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    column.set_size_request(200, -1);
    column.set_margin_top(8);
    column.set_margin_start(8);
    column.set_margin_end(4);
    column.append(&gtk4::Label::builder().label("omafil").xalign(0.0).css_classes(["brand"]).build());

    if let Ok(home) = paths::current_user_home_directory() {
        column.append(&shortcut(state, "Home", "user-home", home));
    }
    column.append(&shortcut(state, "Filesystem", "drive-harddisk", PathBuf::from("/")));

    let saved = store::read_state();
    if !saved.pins.is_empty() {
        column.append(&heading("Pinned"));
        for pin in &saved.pins {
            column.append(&shortcut(state, &pin.label, "folder", PathBuf::from(&pin.path)));
        }
    }

    column.append(&heading("Your Files"));
    for (location, label) in USER_DIRECTORIES {
        if let Ok(path) = paths::known_directory_path(location) {
            column.append(&shortcut(state, label, file_icons::folder_icon_name(location), path));
        }
    }

    let found = drives::read_drives();
    if !found.is_empty() {
        column.append(&heading("Drives"));
        for drive in &found {
            let free = drive.available_bytes;
            let icon = if drive.is_removable { "media-removable" } else { "drive-harddisk" };
            let button = shortcut(state, &drive.name, icon, PathBuf::from(&drive.mount_point));
            button.set_tooltip_text(Some(&format!("{} free of {}", format_bytes(free), format_bytes(drive.total_bytes))));
            column.append(&button);
        }
    }

    if !saved.tags.is_empty() {
        column.append(&heading("Tags"));
        for tag in &saved.tags {
            let entry = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
            let dot = gtk4::Label::new(None);
            // Tag colours come from saved state, so they are escaped before
            // being placed in markup.
            dot.set_markup(&format!("<span color='{}'>●</span>", glib::markup_escape_text(&tag.color)));
            entry.append(&dot);
            entry.append(&gtk4::Label::builder().label(&tag.label).xalign(0.0).build());
            column.append(&entry);
        }
    }

    gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never).child(&column).build().upcast()
}

const ROW_ICON_PX: i32 = 24;

/// Thumbnails come from `core::thumbnail`, which reads the shared freedesktop
/// cache and otherwise runs an installed thumbnailer. Both can block, so the
/// lookup runs off the main thread and the result is dropped if the row has
/// been recycled onto a different file in the meantime.
fn request_thumbnail(image: &gtk4::Image, row: &Row) {
    if row.is_directory() {
        return;
    }
    let path = row.path();
    if let Some(known) = row.imp().thumbnail.borrow().as_ref() {
        if let Some(found) = known {
            image.set_from_file(Some(found));
        }
        return;
    }

    let image = image.clone();
    let row = row.clone();
    let requested = path.clone();
    glib::spawn_future_local(async move {
        let found = gio::spawn_blocking(move || thumbnail::thumbnail(path).ok()).await.ok().flatten();
        row.imp().thumbnail.replace(Some(found.clone()));
        // The factory recycles widgets; only paint if this image still shows
        // the file the lookup was started for.
        if image.widget_name() == requested {
            if let Some(found) = found {
                image.set_from_file(Some(&found));
            }
        }
    });
}

fn name_column() -> gtk4::ColumnViewColumn {
    let factory = gtk4::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let cell = gtk4::Box::builder().orientation(gtk4::Orientation::Horizontal).spacing(8).css_classes(["cell"]).build();
        let image = gtk4::Image::new();
        image.set_pixel_size(ROW_ICON_PX);
        cell.append(&image);
        cell.append(&gtk4::Label::builder().xalign(0.0).ellipsize(gtk4::pango::EllipsizeMode::Middle).build());
        item.downcast_ref::<gtk4::ListItem>().expect("list item").set_child(Some(&cell));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk4::ListItem>().expect("list item");
        let Some(entry) = item.item().and_downcast::<Row>() else { return };
        let Some(cell) = item.child().and_downcast::<gtk4::Box>() else { return };
        let Some(image) = cell.first_child().and_downcast::<gtk4::Image>() else { return };
        let Some(label) = image.next_sibling().and_downcast::<gtk4::Label>() else { return };
        image.set_icon_name(Some(&entry.imp().icon.borrow()));
        image.set_widget_name(&entry.path());
        label.set_text(&entry.imp().name.borrow());
        request_thumbnail(&image, &entry);
    });
    let column = gtk4::ColumnViewColumn::new(Some("Name"), Some(factory));
    column.set_expand(true);
    column
}

fn text_column(title: &str, expand: bool, read: fn(&Row) -> String) -> gtk4::ColumnViewColumn {
    let factory = gtk4::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let label = gtk4::Label::builder().xalign(0.0).css_classes(["cell"]).build();
        item.downcast_ref::<gtk4::ListItem>().expect("list item").set_child(Some(&label));
    });
    factory.connect_bind(move |_, item| {
        let item = item.downcast_ref::<gtk4::ListItem>().expect("list item");
        let Some(entry) = item.item().and_downcast::<Row>() else { return };
        let Some(label) = item.child().and_downcast::<gtk4::Label>() else { return };
        label.set_text(&read(&entry));
    });
    let column = gtk4::ColumnViewColumn::new(Some(title), Some(factory));
    column.set_expand(expand);
    column
}

fn ask_for_name(parent: &gtk4::ApplicationWindow, current: &str, on_accept: impl Fn(String) + 'static) {
    let entry = gtk4::Entry::builder().text(current).activates_default(true).build();
    let dialog = gtk4::Window::builder()
        .transient_for(parent).modal(true).title("Rename").default_width(360).build();
    let confirm = gtk4::Button::builder().label("Rename").build();
    let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let cancel = gtk4::Button::builder().label("Cancel").build();
    buttons.append(&cancel);
    buttons.append(&confirm);
    let content = gtk4::Box::builder().orientation(gtk4::Orientation::Vertical).spacing(12).margin_top(12)
        .margin_bottom(12).margin_start(12).margin_end(12).build();
    content.append(&entry);
    content.append(&buttons);
    dialog.set_child(Some(&content));

    let closing = dialog.clone();
    cancel.connect_clicked(move |_| closing.close());
    let closing = dialog.clone();
    confirm.connect_clicked(move |_| {
        let name = entry.text().to_string();
        if !name.trim().is_empty() {
            on_accept(name);
        }
        closing.close();
    });
    dialog.present();
}

fn install_actions(app: &Rc<App>, window: &gtk4::ApplicationWindow) {
    let actions = gio::SimpleActionGroup::new();

    let open = gio::SimpleAction::new("open", None);
    let state = Rc::clone(app);
    open.connect_activate(move |_, _| {
        if let Some(row) = state.selected_rows().first() {
            if row.is_directory() {
                navigate(&state, PathBuf::from(row.path()), true);
            }
        }
    });
    actions.add_action(&open);

    let terminal = gio::SimpleAction::new("terminal", None);
    let state = Rc::clone(app);
    terminal.connect_activate(move |_, _| {
        let here = state.pane().folder.borrow().to_string_lossy().into_owned();
        if let Err(error) = launch::open_terminal(&here) {
            state.status.set_text(&format!("{error:?}"));
        }
    });
    actions.add_action(&terminal);

    let rename = gio::SimpleAction::new("rename", None);
    let state = Rc::clone(app);
    let parent = window.clone();
    rename.connect_activate(move |_, _| {
        let Some(row) = state.selected_rows().first().cloned() else { return };
        let current = row.imp().name.borrow().clone();
        let state = Rc::clone(&state);
        ask_for_name(&parent, &current, move |name| {
            match operations::rename_entry(row.path(), name) {
                Ok(_) => {
                    let here = state.pane().folder.borrow().clone();
                    navigate(&state, here, false);
                }
                Err(error) => state.status.set_text(&format!("{error:?}")),
            }
        });
    });
    actions.add_action(&rename);

    let open_tab = gio::SimpleAction::new("new-tab", None);
    let opening = Rc::clone(app);
    open_tab.connect_activate(move |_, _| {
        let here = opening.pane().folder.borrow().clone();
        add_tab(&opening, here);
    });
    actions.add_action(&open_tab);

    let shut_tab = gio::SimpleAction::new("close-tab", None);
    let closing = Rc::clone(app);
    shut_tab.connect_activate(move |_, _| {
        let index = closing.active.get();
        close_tab(&closing, index);
    });
    actions.add_action(&shut_tab);

    window.insert_action_group("row", Some(&actions));
    if let Some(application) = window.application() {
        application.set_accels_for_action("row.new-tab", &["<Control>t"]);
        application.set_accels_for_action("row.close-tab", &["<Control>w"]);
    }
}

fn attach_context_menu(state: &Rc<App>, view: &gtk4::ColumnView) {
    let _ = state;
    let menu = gio::Menu::new();
    menu.append(Some("Open"), Some("row.open"));
    menu.append(Some("Rename"), Some("row.rename"));
    menu.append(Some("Open Terminal Here"), Some("row.terminal"));
    let popover = gtk4::PopoverMenu::from_model(Some(&menu));
    popover.set_has_arrow(false);
    popover.set_parent(view);

    let gesture = gtk4::GestureClick::new();
    gesture.set_button(gtk4::gdk::BUTTON_SECONDARY);
    gesture.connect_pressed(move |_, _, x, y| {
        popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.popup();
    });
    view.add_controller(gesture);
}

fn build_window(app: &gtk4::Application, folder: PathBuf) {
    let crumbs = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    let status = gtk4::Label::builder().xalign(0.0).build();
    let back = gtk4::Button::builder().label("Back").sensitive(false).build();
    let up = gtk4::Button::builder().label("Up").build();
    let tabs = gtk4::Box::builder().orientation(gtk4::Orientation::Horizontal).spacing(4).build();
    let stack = gtk4::Stack::new();

    let state = Rc::new(App {
        _watchers: RefCell::new(Vec::new()),
        shortcuts: RefCell::new(Vec::new()),
        panes: RefCell::new(Vec::new()),
        active: Cell::new(0),
        stack: stack.clone(),
        tabs: tabs.clone(),
        crumbs: crumbs.clone(),
        status: status.clone(),
        back: back.clone(),
    });

    let navigating = Rc::clone(&state);
    back.connect_clicked(move |_| {
        let previous = navigating.pane().history.borrow_mut().pop();
        if let Some(previous) = previous {
            navigate(&navigating, previous, false);
        }
    });
    let navigating = Rc::clone(&state);
    up.connect_clicked(move |_| {
        let parent = navigating.pane().folder.borrow().parent().map(Path::to_path_buf);
        if let Some(parent) = parent {
            navigate(&navigating, parent, true);
        }
    });

    let opening = Rc::clone(&state);
    let new_tab = gtk4::Button::builder().icon_name("list-add-symbolic").has_frame(false).tooltip_text("New tab (Ctrl+T)").build();
    new_tab.connect_clicked(move |_| {
        let here = opening.pane().folder.borrow().clone();
        add_tab(&opening, here);
    });

    let sidebar = build_sidebar(&state);

    // The same watchers the Tauri build parks in application state.
    if let Some(theme_watcher) = omarchy::watch_theme(|| {}) {
        state._watchers.borrow_mut().push(Box::new(theme_watcher));
    }
    let by_path = PathBuf::from("/dev/disk/by-path");
    if by_path.is_dir() {
        if let Ok(drive_watcher) = watcher::start_watch(by_path, |_| {}) {
            state._watchers.borrow_mut().push(Box::new(drive_watcher));
        }
    }

    let tab_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    tab_bar.set_margin_top(6);
    tab_bar.set_margin_start(6);
    tab_bar.append(&tabs);
    tab_bar.append(&new_tab);

    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    header.set_margin_start(6);
    header.set_margin_end(6);
    header.append(&back);
    header.append(&up);
    header.append(&crumbs);

    let layout = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    layout.append(&tab_bar);
    layout.append(&header);
    layout.append(&stack);
    stack.set_vexpand(true);
    status.set_margin_start(8);
    status.set_margin_bottom(6);
    layout.append(&status);

    let split = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    split.append(&sidebar);
    split.append(&layout);
    layout.set_hexpand(true);

    let window = gtk4::ApplicationWindow::builder()
        .application(app).title("omafil").default_width(1200).default_height(840).child(&split).build();
    install_actions(&state, &window);
    add_tab(&state, folder);
    window.present();
}

use std::path::Path;

fn main() -> glib::ExitCode {
    let requested = std::env::args().nth(1);
    let application = gtk4::Application::builder().application_id("dev.omafil.Gtk").build();

    // FileManager1 on its own thread, exactly as the Tauri build starts it. The
    // name is requested with DoNotQueue, so an existing file manager keeps it.
    std::thread::spawn(|| {
        if let Ok(connection) = file_manager_service::serve(std::sync::Arc::new(|_request| Ok(()))) {
            std::mem::forget(connection);
        }
    });

    application.connect_activate(move |app| {
        apply_theme();
        let folder = requested.clone().map(PathBuf::from).unwrap_or_else(glib::home_dir);
        build_window(app, folder);
    });

    application.run_with_args::<&str>(&[])
}
