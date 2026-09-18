//! Toolkit spike: what a GTK4 file list costs to start and to hold in memory.
//! Deliberately the idiomatic path — DirectoryList feeding a ListView — so the
//! numbers reflect what a real port would do, not a hand-rolled shortcut.
use gtk4::{gio, glib, prelude::*};

const ATTRS: &str = "standard::name,standard::icon,standard::type,standard::size";

fn build_list(folder: &str) -> gtk4::Widget {
    let directory = gtk4::DirectoryList::new(Some(ATTRS), Some(&gio::File::for_path(folder)));
    let factory = gtk4::SignalListItemFactory::new();

    factory.connect_setup(|_, item| {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        row.append(&gtk4::Image::new());
        row.append(&gtk4::Label::builder().xalign(0.0).build());
        item.downcast_ref::<gtk4::ListItem>()
            .expect("factory items are list items")
            .set_child(Some(&row));
    });

    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk4::ListItem>().expect("factory items are list items");
        let Some(info) = item.item().and_downcast::<gio::FileInfo>() else { return };
        let Some(row) = item.child().and_downcast::<gtk4::Box>() else { return };
        let Some(image) = row.first_child().and_downcast::<gtk4::Image>() else { return };
        let Some(label) = image.next_sibling().and_downcast::<gtk4::Label>() else { return };

        if let Some(icon) = info.icon() {
            image.set_from_gicon(&icon);
        }
        label.set_text(&info.name().to_string_lossy());
    });

    let view = gtk4::ListView::new(Some(gtk4::SingleSelection::new(Some(directory))), Some(factory));
    gtk4::ScrolledWindow::builder().vexpand(true).child(&view).build().upcast()
}

fn main() -> glib::ExitCode {
    let folder = std::env::args().nth(1).unwrap_or_else(|| {
        glib::home_dir().to_string_lossy().into_owned()
    });
    let app = gtk4::Application::builder().application_id("dev.omafil.SpikeGtk4").build();

    app.connect_activate(move |app| {
        gtk4::ApplicationWindow::builder()
            .application(app)
            .title("omafil")
            .default_width(1200)
            .default_height(840)
            .child(&build_list(&folder))
            .build()
            .present();
    });

    // GTK would otherwise try to open the folder argument as a file to load.
    app.run_with_args::<&str>(&[])
}
