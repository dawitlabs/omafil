mod linux_services;
pub mod airdrop;
mod archive;
mod display;
mod drives;
mod diagnostics;
mod desktop_requests;
mod dictation;
mod desktop_integration;
mod file_manager_service;
mod error;
mod icon_theme;
mod inspect;
mod launch;
mod listing;
mod omarchy;
mod openers;
mod operations;
mod operation_queue;
mod operation_io;
mod paths;
mod user_dirs;
mod preview;
mod clipboard;
mod recent;
mod recycle;
mod search;
mod store;
mod thumbnail;
mod watcher;

use crate::drives::{read_drives, DriveInfo, DriveWatcher};
use crate::archive::{create_zip as write_zip, extract_zip as unpack_zip};
use crate::error::{DirectoryError, DriveError, RecentFilesError};
use crate::listing::{
    describe_path as read_path_description, DirectoryEntry,
    DirectoryListing, EntrySort, PathCrumb,
};
use crate::inspect::{inspect_path as read_path_inspection, set_permissions as write_permissions, PathInspection};
use crate::operations::{create_directory, delete_entries, find_transfer_conflicts, permanently_delete_entries, rename_entry, transfer_entries, TransferConflict, TransferConflictPolicy, TransferResult};
use crate::operation_queue::{OperationQueue, QueuedOperation};
use crate::paths::{known_directory_path, resolve_navigable_path};
use crate::recent::{clear_recent_files, read_recent_files, RecentFile};
use crate::recycle::{empty_recycle_bin as purge_recycle_bin, list_recycle_bin, restore_recycle_items as restore_items, RecycleItem};
use crate::search::{search_directory, SearchGeneration, SearchResults};
use crate::store::{read_state, write_state, AppState, StoreError};
use crate::watcher::DirectoryWatcher;
use tauri::{Emitter, Manager, State};

#[derive(Default)]
struct DesktopService(std::sync::Mutex<Option<zbus::blocking::Connection>>);

fn start_desktop_service(app: &tauri::AppHandle, force: bool) -> Result<(), String> {
    let state = app.state::<DesktopService>();
    let mut connection = state.0.lock().map_err(|_| "Desktop service is unavailable.")?;
    if connection.is_some() || (!force && !file_manager_service::is_default()) { return Ok(()); }
    let handle = app.clone();
    *connection = Some(file_manager_service::serve(std::sync::Arc::new(move |request| dispatch_open(&handle, request))).map_err(|e| e.to_string())?);
    Ok(())
}

#[tauri::command]
async fn desktop_integration_status(app: tauri::AppHandle) -> Result<desktop_integration::IntegrationStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut status = desktop_integration::status()?;
        status.service_active = app.state::<DesktopService>().0.lock().map_err(|_| "Desktop service is unavailable.")?.is_some();
        Ok(status)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn configure_desktop_integration(enabled: bool, app: tauri::AppHandle) -> Result<desktop_integration::IntegrationStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if enabled {
            desktop_integration::enable()?;
            // Another file manager can still own the name. Keep the registration
            // for next launch and report that state, rather than killing its owner.
            let _ = start_desktop_service(&app, true);
        } else {
            desktop_integration::restore()?;
            app.state::<DesktopService>().0.lock().map_err(|_| "Desktop service is unavailable.")?.take();
        }
        let mut status = desktop_integration::status()?;
        status.service_active = app.state::<DesktopService>().0.lock().map_err(|_| "Desktop service is unavailable.")?.is_some();
        Ok(status)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn resolve_location(location: String) -> Result<String, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || {
        known_directory_path(&location).map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|_| DirectoryError::unavailable())?
}

#[tauri::command]
async fn list_subdirectories(path: String, show_hidden: bool) -> Result<Vec<PathCrumb>, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || listing::read_subdirectories(&path, show_hidden))
        .await
        .map_err(|_| DirectoryError::read_failed())?
}

#[tauri::command]
async fn list_directory(
    path: String,
    sort: EntrySort,
    descending: bool,
    show_hidden: bool,
    offset: usize,
    limit: usize,
    filter: String,
    reveal_paths: Option<Vec<String>>,
) -> Result<DirectoryListing, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || {
        listing::read_directory_listing_revealing(path, sort, descending, show_hidden, offset, limit, &filter, &reveal_paths.unwrap_or_default())
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
async fn open_terminal(path: String) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || launch::open_terminal(&path))
        .await
        .map_err(|_| DirectoryError::open_failed())?
}

#[tauri::command]
async fn open_in_editor(path: String) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || launch::open_editor(&path))
        .await
        .map_err(|_| DirectoryError::open_failed())?
}

#[tauri::command]
async fn list_openers(path: String) -> Result<Vec<openers::Opener>, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || openers::list_openers(path))
        .await
        .map_err(|_| DirectoryError::open_failed())?
}

#[tauri::command]
async fn open_with(path: String, desktop_id: String) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || openers::open_with(path, desktop_id))
        .await
        .map_err(|_| DirectoryError::open_failed())?
}

#[tauri::command]
async fn mount_drive(device: String) -> Result<String, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || drives::mount_drive(&device))
        .await
        .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn unmount_drive(device: String) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || drives::unmount_drive(&device))
        .await
        .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn format_drive(device: String, filesystem: String, label: String) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || drives::format_drive(&device, &filesystem, &label))
        .await
        .map_err(|_| DirectoryError::detail("The drive could not be formatted."))?
}

#[tauri::command]
async fn eject_drive(device: String) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || drives::eject_drive(&device))
        .await
        .map_err(|_| DirectoryError::operation_failed())?
}

#[tauri::command]
async fn set_default_opener(path: String, desktop_id: String) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || openers::set_default_opener(path, desktop_id))
        .await
        .map_err(|_| DirectoryError::open_failed())?
}

#[tauri::command]
async fn pdf_preview(path: String) -> Result<String, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || preview::pdf_preview(path))
        .await
        .map_err(|_| DirectoryError::read_failed())?
}

#[tauri::command]
fn dictation_status() -> dictation::DictationStatus {
    dictation::dictation_status()
}

#[tauri::command]
fn toggle_dictation() -> Result<(), DirectoryError> {
    dictation::toggle_dictation()
}

#[tauri::command]
async fn thumbnail(path: String) -> Result<String, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || thumbnail::thumbnail(path))
        .await
        .map_err(|_| DirectoryError::read_failed())?
}

#[tauri::command]
async fn preview_asset(path: String, app: tauri::AppHandle) -> Result<String, DirectoryError> {
    let target = tauri::async_runtime::spawn_blocking(move || preview::media_asset(&path))
        .await.map_err(|_| DirectoryError::read_failed())??;
    app.asset_protocol_scope().allow_file(&target)
        .map_err(|_| DirectoryError::detail("Unable to grant access to this preview."))?;
    Ok(target.to_string_lossy().into_owned())
}

#[tauri::command]
fn report_client_error(message: String, detail: Option<String>) {
    if let Some(path) = diagnostics::log_path() {
        diagnostics::append(&path, "client_error", &message, detail.as_deref());
    }
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
async fn read_file_clipboard() -> Result<Option<(Vec<String>, bool)>, DirectoryError> {
    tauri::async_runtime::spawn_blocking(clipboard::read_file_clipboard)
        .await.map_err(|_| DirectoryError::detail("The clipboard could not be read."))?
}

#[tauri::command]
async fn paste_clipboard_image(destination_path: String) -> Result<Option<String>, DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || clipboard::paste_clipboard_image(&destination_path))
        .await.map_err(|_| DirectoryError::detail("The image could not be pasted."))?
}

#[tauri::command]
async fn write_file_clipboard(paths: Vec<String>, is_cut: bool) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || clipboard::write_file_clipboard(&paths, is_cut))
        .await
        .map_err(|_| DirectoryError::detail("The clipboard could not be written."))?
}

#[tauri::command]
async fn trash_paths(paths: Vec<String>) -> Result<Vec<String>, DirectoryError> {
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
async fn set_permissions(path: String, mode: u32) -> Result<(), DirectoryError> {
    tauri::async_runtime::spawn_blocking(move || write_permissions(path, mode))
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
fn watch_directories(
    paths: Vec<String>,
    app: tauri::AppHandle,
    watcher: State<'_, DirectoryWatcher>,
) -> Result<(), DirectoryError> {
    watcher.sync(&paths, move |changed| {
        let _ = app.emit("directory-changed", changed);
    })
}

#[tauri::command]
fn unwatch_directory(watcher: State<'_, DirectoryWatcher>) {
    watcher.stop();
}

#[tauri::command]
fn theme_icons() -> Option<std::collections::HashMap<String, String>> {
    icon_theme::theme_icons()
}

#[tauri::command]
fn take_open_requests(requests: State<'_, desktop_requests::OpenRequests>) -> Vec<desktop_requests::OpenRequest> {
    requests.drain()
}

fn dispatch_open(app: &tauri::AppHandle, request: desktop_requests::OpenRequest) -> Result<(), String> {
    app.state::<desktop_requests::OpenRequests>().push(request)?;
    let _ = app.emit("open-requested", ());
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}

#[tauri::command]
fn read_omarchy_theme() -> Option<omarchy::ThemeColors> {
    omarchy::read_theme()
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
async fn clear_recent_file_history() -> Result<(), RecentFilesError> {
    tauri::async_runtime::spawn_blocking(clear_recent_files)
        .await
        .map_err(|_| RecentFilesError::clear_failed())?
}

#[tauri::command]
async fn list_drives() -> Result<Vec<DriveInfo>, DriveError> {
    tauri::async_runtime::spawn_blocking(read_drives)
        .await
        .map_err(|_| DriveError::discovery_failed())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[tauri::command]
fn display_scale() -> Option<f64> {
    display::compositor_scale()
}

pub fn run() {
    use desktop_requests::{CliAction, OpenRequest, OpenRequests, parse_cli};
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
    let action = parse_cli(&std::env::args().skip(1).collect::<Vec<_>>(), &cwd);
    let mut service_activation = false;
    let initial = match action {
        Ok(CliAction::Help) => {
            println!("Omafil — file manager for Omarchy\n\nUsage: omafil [OPTIONS] [PATH_OR_FILE_URI...]\n\n  --select       Open parent folders and select the supplied items\n  --properties   Show properties for one item\n  --make-default Configure reversible folder and Show in folder integration\n  --restore-default Restore the previous desktop setup\n  --             Treat remaining arguments as literal paths\n  -h, --help     Show this help\n  -V, --version  Show version\n\nSubsequent launches open tabs in the existing window. Remote URIs are not supported yet.");
            return;
        }
        Ok(CliAction::Version) => { println!("omafil {}", env!("CARGO_PKG_VERSION")); return; }
        Ok(CliAction::DesktopService) => { service_activation = true; OpenRequest::default() }
        Ok(CliAction::MakeDefault) => {
            match desktop_integration::enable() {
                Ok(()) => println!("Omafil is configured as the default file manager. Close the previous file manager and restart Omafil to activate Show in folder."),
                Err(error) => { eprintln!("{error}"); std::process::exit(1); }
            }
            return;
        }
        Ok(CliAction::RestoreDefault) => {
            match desktop_integration::restore() {
                Ok(()) => println!("Previous desktop setup restored. Restart Omafil to release its running desktop service."),
                Err(error) => { eprintln!("{error}"); std::process::exit(1); }
            }
            return;
        }
        Ok(CliAction::Open(request)) => request,
        Err(error) => OpenRequest { targets: vec![], error: Some(error) },
    };
    let requests = OpenRequests::default();
    let _ = requests.push(initial);
    display::prefer_xwayland();

    diagnostics::install_panic_hook();
    tauri::Builder::default()
        .manage(requests)
        .manage(linux_services::ServiceRequests::default())
        .manage(DesktopService::default())
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            let request = match parse_cli(&args.into_iter().skip(1).collect::<Vec<_>>(), std::path::Path::new(&cwd)) {
                Ok(CliAction::Open(request)) => request,
                Ok(CliAction::DesktopService) => {
                    let app = app.clone();
                    std::thread::spawn(move || { let _ = start_desktop_service(&app, true); });
                    return;
                }
                Ok(_) => return,
                Err(error) => OpenRequest { targets: vec![], error: Some(error) },
            };
            if let Err(error) = dispatch_open(app, request) {
                let _ = app.emit("open-request-error", error);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let desktop_handle = app.handle().clone();
            std::thread::spawn(move || {
                if let Err(error) = start_desktop_service(&desktop_handle, service_activation) {
                        if let Some(path) = diagnostics::log_path() {
                            diagnostics::append(&path, "desktop_service", "FileManager1 is unavailable", Some(&error.to_string()));
                        }
                }
            });
            app.manage(DirectoryWatcher::default());
            app.manage(SearchGeneration::default());
            app.manage(OperationQueue::default());

            let handle = app.handle().clone();
            if let Some(theme_watcher) = omarchy::watch_theme(move || {
                let _ = handle.emit("omarchy-theme-changed", ());
            }) {
                app.manage(theme_watcher);
            }

            // udev keeps this directory in step with attached block devices.
            let by_path = std::path::PathBuf::from("/dev/disk/by-path");
            let drive_handle = app.handle().clone();
            if by_path.is_dir() {
                if let Ok(drive_watcher) = crate::watcher::start_watch(by_path, move |_| {
                    let _ = drive_handle.emit("drives-changed", ());
                }) {
                    app.manage(DriveWatcher(drive_watcher));
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            linux_services::linux_service,
            linux_services::cancel_linux_service,
            desktop_integration_status,
            configure_desktop_integration,
            airdrop::commands::airdrop_availability,
            airdrop::commands::discover_airdrop_devices,
            airdrop::commands::send_airdrop_file,
            airdrop::commands::accept_airdrop_transfer,
            airdrop::commands::reject_airdrop_transfer,
            airdrop::commands::cancel_airdrop_transfer,
            resolve_location,
            list_directory,
            list_subdirectories,
            open_path,
            open_terminal,
            open_in_editor,
            list_openers,
            open_with,
            set_default_opener,
            pdf_preview,
            thumbnail,
            dictation_status,
            toggle_dictation,
            preview_asset,
            report_client_error,
            mount_drive,
            unmount_drive,
            eject_drive,
            format_drive,
            new_directory,
            rename_path,
            read_file_clipboard,
            paste_clipboard_image,
            write_file_clipboard,
            trash_paths,
            transfer_paths,
            queue_transfer,
            cancel_operation,
            queue_create_zip,
            queue_extract_zip,
            permanently_delete_paths,
            create_zip,
            extract_zip,
            display_scale,
            list_recycle_items,
            restore_recycle_items,
            empty_recycle_bin,
            transfer_conflicts,
            inspect_path,
            set_permissions,
            describe_path,
            search_files,
            watch_directories,
            unwatch_directory,
            read_omarchy_theme,
            take_open_requests,
            theme_icons,
            load_state,
            save_state,
            list_recent_files,
            clear_recent_file_history,
            list_drives
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
