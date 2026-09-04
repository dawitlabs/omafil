mod drives;
mod error;
mod listing;
mod operations;
mod paths;
mod recent;
mod search;
mod store;
mod watcher;

use crate::drives::{read_drives, DriveInfo};
use crate::error::{DirectoryError, DriveError, RecentFilesError};
use crate::listing::{
    describe_path as read_path_description, read_directory_listing, DirectoryEntry,
    DirectoryListing, EntrySort,
};
use crate::operations::{create_directory, delete_entries, rename_entry, transfer_entries};
use crate::paths::{known_directory_path, resolve_navigable_path};
use crate::recent::{read_recent_files, RecentFile};
use crate::search::{search_directory, SearchResults};
use crate::store::{read_state, write_state, AppState, StoreError};
use crate::watcher::DirectoryWatcher;
use tauri::{Emitter, Manager, State};

#[tauri::command]
async fn resolve_location(location: String) -> Result<String, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || {
        known_directory_path(&location).map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|_| DirectoryError::unavailable())?
}

#[tauri::command]
async fn list_directory(
    path: String,
    sort: EntrySort,
    descending: bool,
    show_hidden: bool,
) -> Result<DirectoryListing, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || {
        read_directory_listing(path, sort, descending, show_hidden)
    })
        .await
        .map_err(|_| DirectoryError::read_failed())?
}

#[tauri::command]
async fn open_path(path: String) -> Result<(), DirectoryError> {
    let target = tauri::async_runtime::spawn_blocking(move || resolve_navigable_path(&path))
        .await
        .map_err(|_| DirectoryError::open_failed())??;

    tauri_plugin_opener::open_path(target, None::<&str>).map_err(|_| DirectoryError::open_failed())
}

#[tauri::command]
async fn new_directory(parent_path: String, name: String) -> Result<String, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || create_directory(parent_path, name))
        .await
        .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn rename_path(path: String, name: String) -> Result<String, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || rename_entry(path, name))
        .await
        .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn trash_paths(paths: Vec<String>) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || delete_entries(paths))
        .await
        .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn transfer_paths(
    paths: Vec<String>,
    destination_path: String,
    is_move: bool,
) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || {
        transfer_entries(paths, destination_path, is_move)
    })
    .await
    .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn describe_path(path: String) -> Result<DirectoryEntry, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || read_path_description(path))
        .await
        .map_err(|_| DirectoryError::unavailable())?
}

#[tauri::command]
async fn search_files(
    path: String,
    query: String,
    show_hidden: bool,
) -> Result<SearchResults, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || search_directory(path, query, show_hidden))
        .await
        .map_err(|_| DirectoryError::read_failed())?
}

#[tauri::command]
fn watch_directory(
    path: String,
    app: tauri::AppHandle,
    watcher: State<'_, DirectoryWatcher>,
) -> Result<(), DirectoryError> {
    watcher.watch(&path, move |changed| {
        let _ = app.emit("directory-changed", changed);
    })
}

#[tauri::command]
fn unwatch_directory(watcher: State<'_, DirectoryWatcher>) {
    watcher.stop();
}

#[tauri::command]
async fn load_state() -> AppState {
    tauri::async_runtime::spawn_blocking(read_state)
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn save_state(state: AppState) -> Result<(), StoreError> {
    tauri::async_runtime::spawn_blocking(move || write_state(state))
        .await
        .map_err(|_| StoreError::write_failed())?
}

#[tauri::command]
async fn list_recent_files() -> Result<Vec<RecentFile>, RecentFilesError> {
    tauri::async_runtime::spawn_blocking(read_recent_files)
        .await
        .map_err(|_| RecentFilesError::unavailable())?
}

#[tauri::command]
async fn list_drives() -> Result<Vec<DriveInfo>, DriveError> {
    tauri::async_runtime::spawn_blocking(read_drives)
        .await
        .map_err(|_| DriveError::discovery_failed())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(DirectoryWatcher::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            resolve_location,
            list_directory,
            open_path,
            new_directory,
            rename_path,
            trash_paths,
            transfer_paths,
            describe_path,
            search_files,
            watch_directory,
            unwatch_directory,
            load_state,
            save_state,
            list_recent_files,
            list_drives
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
