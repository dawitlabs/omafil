//! Toolkit spike, vertical slice: omafil's real listing and theme code behind a
//! GTK4 file list. The backend modules are compiled from `src-tauri/src`
//! directly rather than copied, so this measures the code that would actually
//! ship, not a reimplementation of it.
//!
//! Still missing everything else: operations, search, thumbnails, tabs, split
//! panes, inspector, drives, D-Bus. Read it as a realistic floor.
#[path = "../../../src-tauri/src/error.rs"]
mod error;
#[path = "../../../src-tauri/src/user_dirs.rs"]
mod user_dirs;
#[path = "../../../src-tauri/src/paths.rs"]
mod paths;
#[path = "../../../src-tauri/src/desktop_requests.rs"]
mod desktop_requests;
#[path = "../../../src-tauri/src/listing.rs"]
mod listing;
#[path = "../../../src-tauri/src/watcher.rs"]
mod watcher;
#[path = "../../../src-tauri/src/omarchy.rs"]
mod omarchy;

use gtk4::{gio, glib, prelude::*, subclass::prelude::*};
use listing::{DirectoryEntry, DirectoryEntryType, EntrySort};

mod row {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    pub struct Inner {
        pub name: RefCell<String>,
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
        inner.is_directory.set(is_directory);
        inner.size.replace(if is_directory { String::new() } else { format_bytes(entry.size) });
        inner.modified.replace(entry.modified.map(format_modified).unwrap_or_default());
        object
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

/// Omarchy's palette applied as GTK CSS. The webview build does the same thing
/// with custom properties; GTK takes named colours the same way.
fn apply_theme() {
    let Some(colors) = omarchy::read_theme() else { return };
    let pick = |key: &str, fallback: &str| colors.get(key).cloned().unwrap_or_else(|| fallback.to_owned());
    let css = format!(
        "window, listview {{ background: {background}; color: {foreground}; }}
         columnview listview > row:selected {{ background: {accent}; }}
         columnview header button {{ background: {background}; color: {foreground}; }}
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

fn build_view(folder: &std::path::Path) -> gtk4::Widget {
    let store = gio::ListStore::new::<Row>();
    // The same entry point the Tauri command calls; the simpler wrappers beside
    // it are test-only.
    let listed = listing::read_directory_listing_revealing(
        folder.to_string_lossy().into_owned(),
        EntrySort::Name,
        false,
        false,
        0,
        listing::MAX_PAGE_SIZE,
        "",
        &[],
    );
    match listed {
        Ok(listing) => {
            for entry in &listing.entries {
                store.append(&Row::new(entry));
            }
        }
        Err(error) => {
            let message = gtk4::Label::new(Some(&format!("{folder:?} could not be read: {error:?}")));
            return message.upcast();
        }
    }

    let view = gtk4::ColumnView::new(Some(gtk4::MultiSelection::new(Some(store))));
    view.append_column(&text_column("Name", true, |row| row.imp().name.borrow().clone()));
    view.append_column(&text_column("Size", false, |row| row.imp().size.borrow().clone()));
    view.append_column(&text_column("Modified", false, |row| row.imp().modified.borrow().clone()));
    // Arrow keys, Home/End, shift-range and type-ahead come from ColumnView.
    view.set_enable_rubberband(true);

    gtk4::ScrolledWindow::builder().vexpand(true).child(&view).build().upcast()
}

fn main() -> glib::ExitCode {
    let requested = std::env::args().nth(1);
    let app = gtk4::Application::builder().application_id("dev.omafil.SpikeGtk4").build();

    app.connect_activate(move |app| {
        apply_theme();
        let folder = requested
            .clone()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| glib::home_dir());
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("omafil")
            .default_width(1200)
            .default_height(840)
            .child(&build_view(&folder))
            .build();
        window.present();
    });

    app.run_with_args::<&str>(&[])
}
