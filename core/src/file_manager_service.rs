//! Standard Linux "show in file manager" interface, independent of the webview.
use crate::desktop_requests::{request_for, OpenMode, OpenRequest};
use std::{path::Path, sync::Arc};

type Handler = Arc<dyn Fn(OpenRequest) -> Result<(), String> + Send + Sync>;
struct FileManager {
    handler: Handler,
}

impl FileManager {
    fn dispatch(&self, uris: Vec<String>, mode: OpenMode) -> zbus::fdo::Result<()> {
        if uris.is_empty() {
            return Err(zbus::fdo::Error::InvalidArgs(
                "Provide at least one URI.".into(),
            ));
        }
        if uris.iter().any(|uri| !uri.starts_with("file:")) {
            return Err(zbus::fdo::Error::NotSupported(
                "Only local file:// URIs are supported.".into(),
            ));
        }
        let request =
            request_for(&uris, Path::new("/"), mode).map_err(zbus::fdo::Error::InvalidArgs)?;
        (self.handler)(request).map_err(zbus::fdo::Error::Failed)
    }
}

#[zbus::interface(name = "org.freedesktop.FileManager1")]
impl FileManager {
    fn show_folders(&self, uris: Vec<String>, _startup_id: &str) -> zbus::fdo::Result<()> {
        self.dispatch(uris, OpenMode::Folders)
    }
    fn show_items(&self, uris: Vec<String>, _startup_id: &str) -> zbus::fdo::Result<()> {
        self.dispatch(uris, OpenMode::Select)
    }
    fn show_item_properties(&self, uris: Vec<String>, _startup_id: &str) -> zbus::fdo::Result<()> {
        self.dispatch(uris, OpenMode::Properties)
    }
}

pub fn serve(handler: Handler) -> zbus::Result<zbus::blocking::Connection> {
    let connection = zbus::blocking::connection::Builder::session()?
        .serve_at("/org/freedesktop/FileManager1", FileManager { handler })?
        .build()?;
    // Never replace or wait behind another file manager: activation is opt-in
    // through the desktop default and this name belongs to the current owner.
    connection.request_name_with_flags(
        "org.freedesktop.FileManager1",
        zbus::fdo::RequestNameFlags::DoNotQueue.into(),
    )?;
    Ok(connection)
}

pub fn is_default() -> bool {
    std::process::Command::new("xdg-mime")
        .args(["query", "default", "inode/directory"])
        .output()
        .is_ok_and(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).trim() == "omafil.desktop"
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Run explicitly on a disposable bus so tests never claim a desktop service
    // on the user's real session: dbus-run-session -- cargo test desktop_bus -- --ignored
    #[test]
    #[ignore = "requires an isolated session bus"]
    fn desktop_bus_routes_methods_and_refuses_to_replace_an_owner() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a file.txt");
        std::fs::write(&path, "fixture").unwrap();
        let (send, receive) = std::sync::mpsc::channel();
        let _server = serve(Arc::new(move |request| {
            send.send(request).map_err(|e| e.to_string())
        }))
        .unwrap();
        assert!(serve(Arc::new(|_| Ok(()))).is_err());
        let client = zbus::blocking::Connection::session().unwrap();
        let proxy = zbus::blocking::Proxy::new(
            &client,
            "org.freedesktop.FileManager1",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1",
        )
        .unwrap();
        let uri = url::Url::from_file_path(&path).unwrap().to_string();
        let folder_uri = url::Url::from_directory_path(root.path())
            .unwrap()
            .to_string();
        let _: () = proxy.call("ShowItems", &(vec![uri.clone()], "")).unwrap();
        assert_eq!(
            receive
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
                .targets[0]
                .selection,
            vec![path.to_str().unwrap()]
        );
        let _: () = proxy.call("ShowFolders", &(vec![folder_uri], "")).unwrap();
        assert!(receive
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap()
            .targets[0]
            .selection
            .is_empty());
        let _: () = proxy
            .call("ShowItemProperties", &(vec![uri.clone()], ""))
            .unwrap();
        assert_eq!(
            receive
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
                .targets[0]
                .properties
                .as_deref(),
            path.to_str()
        );
        assert!(proxy
            .call::<_, _, ()>("ShowItems", &(vec!["smb://server/share"], ""))
            .is_err());
        assert!(proxy
            .call::<_, _, ()>("ShowItemProperties", &(vec![uri.clone(), uri], ""))
            .is_err());
    }
}
