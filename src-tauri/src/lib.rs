mod drives;
mod archive;
mod error;
mod inspect;
mod listing;
mod operations;
mod operation_queue;
mod operation_io;
mod paths;
mod recent;
mod recycle;
mod search;
mod store;
mod watcher;

use crate::drives::{read_drives, DriveInfo};
use crate::archive::{create_zip as write_zip, extract_zip as unpack_zip};
use crate::error::{DirectoryError, DriveError, RecentFilesError};
use crate::listing::{
    describe_path as read_path_description, read_directory_listing, DirectoryEntry,
    DirectoryListing, EntrySort,
};
use crate::inspect::{inspect_path as read_path_inspection, PathInspection};
use crate::operations::{create_directory, delete_entries, find_transfer_conflicts, permanently_delete_entries, rename_entry, transfer_entries, TransferConflict, TransferConflictPolicy, TransferResult};
use crate::operation_queue::{OperationQueue, QueuedOperation};
use crate::paths::{known_directory_path, resolve_navigable_path};
use crate::recent::{read_recent_files, RecentFile};
use crate::recycle::{empty_recycle_bin as purge_recycle_bin, list_recycle_bin, restore_recycle_items as restore_items, RecycleItem};
use crate::search::{search_directory, SearchGeneration, SearchResults};
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
    offset: usize,
    limit: usize,
) -> Result<DirectoryListing, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || {
        read_directory_listing(path, sort, descending, show_hidden, offset, limit)
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
    conflict_policy: TransferConflictPolicy,
) -> Result<Vec<TransferResult>, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || {
        transfer_entries(paths, destination_path, is_move, conflict_policy)
    })
    .await
    .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
fn queue_transfer(
    paths: Vec<String>,
    destination_path: String,
    is_move: bool,
    conflict_policy: TransferConflictPolicy,
    app: tauri::AppHandle,
    queue: State<'_, OperationQueue>,
) -> QueuedOperation {
    queue.start_transfer(app, paths, destination_path, is_move, conflict_policy)
}

#[tauri::command]
fn cancel_operation(id: String, queue: State<'_, OperationQueue>) -> bool {
    queue.cancel(&id)
}

#[tauri::command]
fn queue_create_zip(paths: Vec<String>, destination_path: String, name: String, app: tauri::AppHandle, queue: State<'_, OperationQueue>) -> QueuedOperation {
    queue.start_archive_create(app, paths, destination_path, name)
}

#[tauri::command]
fn queue_extract_zip(path: String, destination_path: String, app: tauri::AppHandle, queue: State<'_, OperationQueue>) -> QueuedOperation {
    queue.start_archive_extract(app, path, destination_path)
}

#[tauri::command]
async fn permanently_delete_paths(paths: Vec<String>) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || permanently_delete_entries(paths))
        .await
        .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn create_zip(paths: Vec<String>, destination_path: String, name: String) -> Result<String, DirectoryError> { tauri::async_runtime::spawn_blocking(move || write_zip(paths, destination_path, name)).await.map_err(|_| DirectoryError::operation_failed())? }

#[tauri::command]
async fn extract_zip(path: String, destination_path: String) -> Result<(), DirectoryError> { tauri::async_runtime::spawn_blocking(move || unpack_zip(path, destination_path)).await.map_err(|_| DirectoryError::operation_failed())? }

#[tauri::command]
async fn list_recycle_items() -> Result<Vec<RecycleItem>, DirectoryError> {
    tauri::async_runtime::spawn_blocking(list_recycle_bin)
        .await
        .map_err(|_| DirectoryError::unavailable())?
}

#[tauri::command]
async fn restore_recycle_items(ids: Vec<String>) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || restore_items(ids))
        .await
        .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn empty_recycle_bin() -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(purge_recycle_bin)
        .await
        .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn transfer_conflicts(paths: Vec<String>, destination_path: String) -> Result<Vec<TransferConflict>, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || find_transfer_conflicts(paths, destination_path))
        .await
        .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn inspect_path(path: String) -> Result<PathInspection, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || read_path_inspection(path))
        .await
        .map_err(|_| DirectoryError::unavailable())?
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
    generation: State<'_, SearchGeneration>,
) -> Result<SearchResults, DirectoryError> {
    let (current, id) = generation.begin();

    tauri::async_runtime::spawn_blocking(move || {
        search_directory(path, query, show_hidden, id, current)
    })
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
            app.manage(SearchGeneration::default());
            app.manage(OperationQueue::default());
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
            queue_transfer,
            cancel_operation,
            queue_create_zip,
            queue_extract_zip,
            permanently_delete_paths,
            create_zip,
            extract_zip,
            list_recycle_items,
            restore_recycle_items,
            empty_recycle_bin,
            transfer_conflicts,
            inspect_path,
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
