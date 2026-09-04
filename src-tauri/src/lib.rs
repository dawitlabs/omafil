mod drives;
mod error;
mod listing;
mod operations;
mod paths;
mod recent;

use crate::drives::{read_drives, DriveInfo};
use crate::error::{DirectoryError, DriveError, RecentFilesError};
use crate::listing::{read_directory_listing, DirectoryListing, EntrySort};
use crate::operations::{create_directory, delete_entries, rename_entry, transfer_entries};
use crate::paths::{known_directory_path, resolve_navigable_path};
use crate::recent::{read_recent_files, RecentFile};

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
) -> Result<DirectoryListing, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || read_directory_listing(path, sort, descending))
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
        .invoke_handler(tauri::generate_handler![
            resolve_location,
            list_directory,
            open_path,
            new_directory,
            rename_path,
            trash_paths,
            transfer_paths,
            list_recent_files,
            list_drives
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
