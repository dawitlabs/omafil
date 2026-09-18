//! GTK4 front end for omafil, built on the same `omafil-core` backend as the
//! Tauri build. See `docs/gtk-port.md` for what is ported and what is not.
//!
//! Destructive operations are not wired yet: until the undo and recycle
//! surfaces are ported, this build cannot delete anything.

use gtk4::{gio, glib, prelude::*, subclass::prelude::*};
use omafil_core::{drives, file_icons, file_manager_service, icon_theme, launch, listing, omarchy, operations, paths, store, watcher};
use listing::{DirectoryEntry, DirectoryEntryType, EntrySort};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

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

struct App {
    _watchers: RefCell<Vec<Box<dyn std::any::Any>>>,
    /// Sidebar entries by the path they open, so the current folder can be
    /// marked without rebuilding the list.
    shortcuts: RefCell<Vec<(PathBuf, gtk4::Button)>>,
    folder: RefCell<PathBuf>,
    history: RefCell<Vec<PathBuf>>,
    store: gio::ListStore,
    selection: gtk4::MultiSelection,
    crumbs: gtk4::Box,
    status: gtk4::Label,
    back: gtk4::Button,
}

impl App {
    fn selected_rows(&self) -> Vec<Row> {
        let bitset = self.selection.selection();
        (0..bitset.size())
            .filter_map(|index| self.selection.item(bitset.nth(index as u32)))
            .filter_map(|item| item.downcast::<Row>().ok())
            .collect()
    }
}

fn navigate(app: &Rc<App>, target: PathBuf, remember: bool) {
    let listed = listing::read_directory_listing_revealing(
        target.to_string_lossy().into_owned(),
        EntrySort::Name, false, false, 0, listing::MAX_PAGE_SIZE, "", &[],
    );
    let Ok(listing) = listed else {
        app.status.set_text(&format!("{} could not be read.", target.display()));
        return;
    };

    if remember {
        app.history.borrow_mut().push(app.folder.borrow().clone());
    }
    app.folder.replace(target.clone());
    app.store.remove_all();
    for entry in &listing.entries {
        app.store.append(&Row::new(entry));
    }

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
    app.back.set_sensitive(!app.history.borrow().is_empty());
    app.status.set_text(&format!(
        "{} items{}",
        listing.total,
        if listing.has_more { ", showing the first page" } else { "" }
    ));
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

fn name_column() -> gtk4::ColumnViewColumn {
    let factory = gtk4::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let cell = gtk4::Box::builder().orientation(gtk4::Orientation::Horizontal).spacing(8).css_classes(["cell"]).build();
        cell.append(&gtk4::Image::new());
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
        label.set_text(&entry.imp().name.borrow());
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
        let here = state.folder.borrow().to_string_lossy().into_owned();
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
                    let here = state.folder.borrow().clone();
                    navigate(&state, here, false);
                }
                Err(error) => state.status.set_text(&format!("{error:?}")),
            }
        });
    });
    actions.add_action(&rename);

    window.insert_action_group("row", Some(&actions));
}

fn build_window(app: &gtk4::Application, folder: PathBuf) {
    let store = gio::ListStore::new::<Row>();
    let selection = gtk4::MultiSelection::new(Some(store.clone()));
    let view = gtk4::ColumnView::new(Some(selection.clone()));
    view.append_column(&name_column());
    view.append_column(&text_column("Size", false, |row| row.imp().size.borrow().clone()));
    view.append_column(&text_column("Modified", false, |row| row.imp().modified.borrow().clone()));
    view.set_enable_rubberband(true);

    let crumbs = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    let status = gtk4::Label::builder().xalign(0.0).build();
    let back = gtk4::Button::builder().label("Back").sensitive(false).build();
    let up = gtk4::Button::builder().label("Up").build();

    let state = Rc::new(App {
        _watchers: RefCell::new(Vec::new()),
        shortcuts: RefCell::new(Vec::new()),
        folder: RefCell::new(folder.clone()),
        history: RefCell::new(Vec::new()),
        store,
        selection: selection.clone(),
        crumbs: crumbs.clone(),
        status: status.clone(),
        back: back.clone(),
    });

    let navigating = Rc::clone(&state);
    back.connect_clicked(move |_| {
        let previous = navigating.history.borrow_mut().pop();
        if let Some(previous) = previous {
            navigate(&navigating, previous, false);
        }
    });
    let navigating = Rc::clone(&state);
    up.connect_clicked(move |_| {
        let parent = navigating.folder.borrow().parent().map(Path::to_path_buf);
        if let Some(parent) = parent {
            navigate(&navigating, parent, true);
        }
    });

    // Enter and double click both land here.
    let navigating = Rc::clone(&state);
    view.connect_activate(move |view, position| {
        let Some(row) = view.model().and_then(|model| model.item(position)).and_downcast::<Row>() else { return };
        if row.is_directory() {
            navigate(&navigating, PathBuf::from(row.path()), true);
        }
    });

    let counting = Rc::clone(&state);
    selection.connect_selection_changed(move |selection, _, _| {
        let chosen = selection.selection().size();
        if chosen > 0 {
            counting.status.set_text(&format!("{chosen} selected"));
        }
    });

    let menu = gio::Menu::new();
    menu.append(Some("Open"), Some("row.open"));
    menu.append(Some("Rename"), Some("row.rename"));
    menu.append(Some("Open Terminal Here"), Some("row.terminal"));
    let popover = gtk4::PopoverMenu::from_model(Some(&menu));
    popover.set_has_arrow(false);
    popover.set_parent(&view);

    let gesture = gtk4::GestureClick::new();
    gesture.set_button(gtk4::gdk::BUTTON_SECONDARY);
    let showing = popover.clone();
    gesture.connect_pressed(move |_, _, x, y| {
        showing.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        showing.popup();
    });
    view.add_controller(gesture);

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

    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    header.set_margin_top(6);
    header.set_margin_start(6);
    header.set_margin_end(6);
    header.append(&back);
    header.append(&up);
    header.append(&crumbs);

    let layout = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    layout.append(&header);
    layout.append(&gtk4::ScrolledWindow::builder().vexpand(true).child(&view).build());
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
    navigate(&state, folder, false);
    window.present();
}

use std::path::Path;

fn main() -> glib::ExitCode {
    let requested = std::env::args().nth(1);
    let application = gtk4::Application::builder().application_id("dev.omafil.SpikeGtk4").build();

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
