use quick_xml::{events::Event, Reader, XmlVersion};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use sysinfo::Disks;
use url::Url;

const MAX_DIRECTORY_ENTRIES: usize = 2000;
const MAX_RECENT_FILES: usize = 50;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryEntry {
    name: String,
    path: String,
    entry_type: DirectoryEntryType,
    size: u64,
    modified: Option<i64>,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum EntrySort {
    Name,
    Size,
    Modified,
    Type,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PathCrumb {
    name: String,
    path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryListing {
    path: String,
    crumbs: Vec<PathCrumb>,
    entries: Vec<DirectoryEntry>,
    total: usize,
}

#[derive(PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum DirectoryEntryType {
    Directory,
    File,
}

#[derive(Serialize)]
struct DirectoryError {
    code: &'static str,
    message: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DriveInfo {
    name: String,
    mount_point: String,
    path: String,
    total_bytes: u64,
    available_bytes: u64,
    is_removable: bool,
    is_read_only: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentFile {
    name: String,
    path: String,
    parent_directory: String,
}

#[derive(Serialize)]
struct DriveError {
    code: &'static str,
    message: &'static str,
}

#[derive(Serialize)]
struct RecentFilesError {
    code: &'static str,
    message: &'static str,
}

impl DirectoryError {
    const fn unavailable() -> Self {
        Self {
            code: "directory_unavailable",
            message: "This folder is unavailable on this device.",
        }
    }

    const fn read_failed() -> Self {
        Self {
            code: "directory_read_failed",
            message: "Unable to read this folder.",
        }
    }

    const fn not_allowed() -> Self {
        Self {
            code: "directory_not_allowed",
            message: "This location is outside your files and drives.",
        }
    }

    const fn open_failed() -> Self {
        Self {
            code: "open_failed",
            message: "Unable to open this item.",
        }
    }

    const fn invalid_name() -> Self {
        Self {
            code: "invalid_name",
            message: "That name contains characters that are not allowed.",
        }
    }

    const fn already_exists() -> Self {
        Self {
            code: "already_exists",
            message: "An item with that name already exists here.",
        }
    }

    const fn invalid_destination() -> Self {
        Self {
            code: "invalid_destination",
            message: "A folder cannot be moved into itself.",
        }
    }

    const fn operation_failed() -> Self {
        Self {
            code: "operation_failed",
            message: "Unable to complete that operation.",
        }
    }
}

impl RecentFilesError {
    const fn unavailable() -> Self {
        Self {
            code: "recent_files_unavailable",
            message: "Unable to read recent files from this desktop.",
        }
    }
}

fn is_supported_location(location: &str) -> bool {
    matches!(
        location,
        "home" | "desktop" | "documents" | "downloads" | "pictures" | "videos" | "music"
    )
}

fn current_user_home_directory() -> Result<PathBuf, DirectoryError> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .ok_or_else(DirectoryError::unavailable)
}

fn known_directory_path(location: &str) -> Result<PathBuf, DirectoryError> {
    if !is_supported_location(location) {
        return Err(DirectoryError::unavailable());
    }

    let home_directory = current_user_home_directory()?;
    let directory = match location {
        "home" => home_directory,
        "desktop" => home_directory.join("Desktop"),
        "documents" => home_directory.join("Documents"),
        "downloads" => home_directory.join("Downloads"),
        "pictures" => home_directory.join("Pictures"),
        "videos" => home_directory.join("Videos"),
        "music" => home_directory.join("Music"),
        _ => return Err(DirectoryError::unavailable()),
    };

    directory
        .is_dir()
        .then_some(directory)
        .ok_or_else(DirectoryError::unavailable)
}

fn entry_extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(stem, extension)| {
            if stem.is_empty() {
                String::new()
            } else {
                extension.to_lowercase()
            }
        })
        .unwrap_or_default()
}

fn compare_entries(
    left: &DirectoryEntry,
    right: &DirectoryEntry,
    sort: EntrySort,
    descending: bool,
) -> std::cmp::Ordering {
    let left_is_directory = left.entry_type == DirectoryEntryType::Directory;
    let right_is_directory = right.entry_type == DirectoryEntryType::Directory;
    let by_name = left.name.to_lowercase().cmp(&right.name.to_lowercase());

    let by_key = match sort {
        EntrySort::Name => by_name,
        EntrySort::Size => left.size.cmp(&right.size).then(by_name),
        EntrySort::Modified => left.modified.cmp(&right.modified).then(by_name),
        EntrySort::Type => entry_extension(&left.name)
            .cmp(&entry_extension(&right.name))
            .then(by_name),
    };

    right_is_directory
        .cmp(&left_is_directory)
        .then(if descending { by_key.reverse() } else { by_key })
}

fn read_directory_entries(
    directory: &Path,
    sort: EntrySort,
    descending: bool,
) -> Result<(Vec<DirectoryEntry>, usize), ()> {
    let directory_entries = fs::read_dir(directory).map_err(|_| ())?;
    let mut entries = Vec::new();

    for directory_entry in directory_entries {
        let directory_entry = directory_entry.map_err(|_| ())?;
        let name = directory_entry.file_name().to_string_lossy().into_owned();

        if name.starts_with('.') {
            continue;
        }

        let file_type = directory_entry.file_type().map_err(|_| ())?;
        let metadata = directory_entry.metadata().ok();

        entries.push(DirectoryEntry {
            path: directory.join(&name).to_string_lossy().into_owned(),
            name,
            entry_type: if file_type.is_dir() {
                DirectoryEntryType::Directory
            } else {
                DirectoryEntryType::File
            },
            size: metadata.as_ref().map_or(0, |data| data.len()),
            modified: metadata
                .as_ref()
                .and_then(|data| data.modified().ok())
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|elapsed| elapsed.as_secs() as i64),
        });
    }

    entries.sort_by(|left, right| compare_entries(left, right, sort, descending));

    let total = entries.len();
    entries.truncate(MAX_DIRECTORY_ENTRIES);

    Ok((entries, total))
}

fn navigable_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(home_directory) = current_user_home_directory()
        .ok()
        .and_then(|path| path.canonicalize().ok())
    {
        roots.push(home_directory);
    }

    for mount_point in Disks::new_with_refreshed_list()
        .list()
        .iter()
        .map(|disk| disk.mount_point())
    {
        if mount_point != Path::new("/") && is_user_visible_drive_mount(mount_point) {
            if let Ok(root) = mount_point.canonicalize() {
                roots.push(root);
            }
        }
    }

    roots
}

fn resolve_navigable_path(path: &str) -> Result<PathBuf, DirectoryError> {
    let requested = Path::new(path)
        .canonicalize()
        .map_err(|_| DirectoryError::unavailable())?;

    navigable_roots()
        .iter()
        .any(|root| requested.starts_with(root))
        .then_some(requested)
        .ok_or_else(DirectoryError::not_allowed)
}

fn path_crumbs(directory: &Path, roots: &[PathBuf]) -> Vec<PathCrumb> {
    let Some(root) = roots
        .iter()
        .filter(|root| directory.starts_with(root))
        .max_by_key(|root| root.components().count())
    else {
        return Vec::new();
    };

    let mut crumbs = vec![PathCrumb {
        name: display_name(root),
        path: root.to_string_lossy().into_owned(),
    }];
    let mut walked = root.to_path_buf();

    for component in directory.strip_prefix(root).unwrap_or(Path::new("")) {
        walked.push(component);
        crumbs.push(PathCrumb {
            name: component.to_string_lossy().into_owned(),
            path: walked.to_string_lossy().into_owned(),
        });
    }

    crumbs
}

fn display_name(directory: &Path) -> String {
    directory
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| directory.to_string_lossy().into_owned())
}

fn read_directory_listing(
    path: String,
    sort: EntrySort,
    descending: bool,
) -> Result<DirectoryListing, DirectoryError> {
    let directory = resolve_navigable_path(&path)?;

    if !directory.is_dir() {
        return Err(DirectoryError::unavailable());
    }

    let (entries, total) = read_directory_entries(&directory, sort, descending)
        .map_err(|_| DirectoryError::read_failed())?;

    Ok(DirectoryListing {
        crumbs: path_crumbs(&directory, &navigable_roots()),
        path: directory.to_string_lossy().into_owned(),
        entries,
        total,
    })
}

fn validate_entry_name(name: &str) -> Result<&str, DirectoryError> {
    let name = name.trim();
    let is_reserved = name.is_empty() || name == "." || name == "..";
    let has_separator = name.contains('/') || name.contains('\\') || name.contains('\0');

    if is_reserved || has_separator {
        return Err(DirectoryError::invalid_name());
    }

    Ok(name)
}

fn vacant_target(directory: &Path, name: &str) -> Result<PathBuf, DirectoryError> {
    let target = directory.join(validate_entry_name(name)?);

    if target.symlink_metadata().is_ok() {
        return Err(DirectoryError::already_exists());
    }

    Ok(target)
}

fn copy_recursively(source: &Path, destination: &Path) -> std::io::Result<()> {
    if !source.symlink_metadata()?.is_dir() {
        return fs::copy(source, destination).map(|_| ());
    }

    fs::create_dir(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        copy_recursively(&entry.path(), &destination.join(entry.file_name()))?;
    }

    Ok(())
}

fn create_directory(parent_path: String, name: String) -> Result<String, DirectoryError> {
    let parent = resolve_navigable_path(&parent_path)?;

    if !parent.is_dir() {
        return Err(DirectoryError::unavailable());
    }

    let target = vacant_target(&parent, &name)?;
    fs::create_dir(&target).map_err(|_| DirectoryError::operation_failed())?;

    Ok(target.to_string_lossy().into_owned())
}

fn rename_entry(path: String, name: String) -> Result<String, DirectoryError> {
    let source = resolve_navigable_path(&path)?;
    let parent = source.parent().ok_or_else(DirectoryError::unavailable)?;
    let target = vacant_target(parent, &name)?;

    fs::rename(&source, &target).map_err(|_| DirectoryError::operation_failed())?;

    Ok(target.to_string_lossy().into_owned())
}

fn delete_entries(paths: Vec<String>) -> Result<(), DirectoryError> {
    let resolved = paths
        .iter()
        .map(|path| resolve_navigable_path(path))
        .collect::<Result<Vec<_>, _>>()?;

    trash::delete_all(&resolved).map_err(|_| DirectoryError::operation_failed())
}

fn transfer_entries(
    paths: Vec<String>,
    destination_path: String,
    is_move: bool,
) -> Result<(), DirectoryError> {
    let destination = resolve_navigable_path(&destination_path)?;

    if !destination.is_dir() {
        return Err(DirectoryError::unavailable());
    }

    for path in &paths {
        let source = resolve_navigable_path(path)?;

        if destination.starts_with(&source) {
            return Err(DirectoryError::invalid_destination());
        }

        let name = source
            .file_name()
            .ok_or_else(DirectoryError::unavailable)?
            .to_string_lossy()
            .into_owned();
        let target = vacant_target(&destination, &name)?;

        if is_move && fs::rename(&source, &target).is_ok() {
            continue;
        }

        copy_recursively(&source, &target).map_err(|_| DirectoryError::operation_failed())?;

        if is_move {
            trash::delete(&source).map_err(|_| DirectoryError::operation_failed())?;
        }
    }

    Ok(())
}

fn recently_used_file_path() -> Result<PathBuf, RecentFilesError> {
    current_user_home_directory()
        .map(|home_directory| home_directory.join(".local/share/recently-used.xbel"))
        .map_err(|_| RecentFilesError::unavailable())
}

fn read_recent_files() -> Result<Vec<RecentFile>, RecentFilesError> {
    let history_path = recently_used_file_path()?;
    if !history_path.exists() {
        return Ok(Vec::new());
    }

    let history = fs::read_to_string(history_path).map_err(|_| RecentFilesError::unavailable())?;
    let mut reader = Reader::from_str(&history);
    let mut recent_files = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) if event.name().as_ref() == b"bookmark" => {
                let mut href = None;
                let mut modified = None;

                for attribute in event.attributes().flatten() {
                    match attribute.key.as_ref() {
                        b"href" => {
                            href = attribute
                                .normalized_value(XmlVersion::Explicit1_0)
                                .ok()
                                .map(|value| value.into_owned())
                        }
                        b"modified" => {
                            modified = attribute
                                .normalized_value(XmlVersion::Explicit1_0)
                                .ok()
                                .map(|value| value.into_owned())
                        }
                        _ => {}
                    }
                }

                let Some((href, modified)) = href.zip(modified) else {
                    continue;
                };
                let Ok(url) = Url::parse(&href) else {
                    continue;
                };
                let Ok(path) = url.to_file_path() else {
                    continue;
                };
                if !path.is_file() {
                    continue;
                }

                let Some(name) = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                else {
                    continue;
                };
                let parent_directory = path
                    .parent()
                    .map(|parent| parent.to_string_lossy().into_owned())
                    .unwrap_or_default();

                recent_files.push((
                    modified,
                    RecentFile {
                        name,
                        path: path.to_string_lossy().into_owned(),
                        parent_directory,
                    },
                ));
            }
            Ok(Event::Eof) => break,
            Err(_) => return Err(RecentFilesError::unavailable()),
            _ => {}
        }
    }

    recent_files.sort_by(|left, right| right.0.cmp(&left.0));
    recent_files.truncate(MAX_RECENT_FILES);
    Ok(recent_files.into_iter().map(|(_, file)| file).collect())
}

fn is_user_visible_drive_mount(mount_point: &Path) -> bool {
    if mount_point == Path::new("/") {
        return true;
    }

    mount_point.starts_with("/media")
        || mount_point.starts_with("/mnt")
        || mount_point.starts_with("/run/media")
}

fn drive_navigation_path(mount_point: &Path) -> PathBuf {
    if mount_point == Path::new("/") {
        if let Ok(home_directory) = current_user_home_directory() {
            return home_directory;
        }
    }

    mount_point.to_path_buf()
}

fn read_drives() -> Vec<DriveInfo> {
    let disks = Disks::new_with_refreshed_list();
    let home_directory_name = current_user_home_directory().ok().and_then(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    });
    let mut drives = disks
        .list()
        .iter()
        .filter(|disk| disk.total_space() > 0 && is_user_visible_drive_mount(disk.mount_point()))
        .map(|disk| DriveInfo {
            name: if disk.mount_point() == Path::new("/") {
                home_directory_name
                    .clone()
                    .unwrap_or_else(|| "Home".to_owned())
            } else {
                disk.name().to_string_lossy().into_owned()
            },
            mount_point: disk.mount_point().to_string_lossy().into_owned(),
            path: drive_navigation_path(disk.mount_point())
                .to_string_lossy()
                .into_owned(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
            is_removable: disk.is_removable(),
            is_read_only: disk.is_read_only(),
        })
        .collect::<Vec<_>>();

    drives.sort_by(|left, right| left.mount_point.cmp(&right.mount_point));
    drives
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
        .map_err(|_| DriveError {
            code: "drive_discovery_failed",
            message: "Unable to discover mounted drives.",
        })
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

#[cfg(test)]
mod tests {
    use super::{
        compare_entries, entry_extension, is_supported_location, is_user_visible_drive_mount,
        path_crumbs, resolve_navigable_path, validate_entry_name, DirectoryEntry,
        DirectoryEntryType, EntrySort,
    };
    use std::path::{Path, PathBuf};

    fn entry(name: &str, is_directory: bool, size: u64) -> DirectoryEntry {
        DirectoryEntry {
            name: name.to_owned(),
            path: format!("/home/dave/{name}"),
            entry_type: if is_directory {
                DirectoryEntryType::Directory
            } else {
                DirectoryEntryType::File
            },
            size,
            modified: Some(size as i64),
        }
    }

    fn sorted(mut entries: Vec<DirectoryEntry>, sort: EntrySort, descending: bool) -> Vec<String> {
        entries.sort_by(|left, right| compare_entries(left, right, sort, descending));
        entries.into_iter().map(|item| item.name).collect()
    }

    #[test]
    fn only_fixed_locations_are_allowed() {
        assert!(is_supported_location("home"));
        assert!(is_supported_location("desktop"));
        assert!(is_supported_location("music"));
        assert!(!is_supported_location("/etc"));
        assert!(!is_supported_location("../Desktop"));
    }

    #[test]
    fn drive_usage_percentage_is_bounded_by_total_space() {
        let total_bytes = 100_u64;
        let available_bytes = 40_u64;
        let used_percentage = (total_bytes.saturating_sub(available_bytes) * 100) / total_bytes;

        assert_eq!(used_percentage, 60);
    }

    #[test]
    fn crumbs_start_at_the_deepest_matching_root() {
        let roots = vec![PathBuf::from("/home/dave"), PathBuf::from("/run/media/usb")];
        let crumbs = path_crumbs(Path::new("/home/dave/Code/omafil"), &roots);

        let trail: Vec<_> = crumbs.iter().map(|crumb| crumb.name.as_str()).collect();
        assert_eq!(trail, ["dave", "Code", "omafil"]);
        assert_eq!(crumbs.last().unwrap().path, "/home/dave/Code/omafil");
    }

    #[test]
    fn crumbs_are_empty_outside_every_root() {
        let roots = vec![PathBuf::from("/home/dave")];

        assert!(path_crumbs(Path::new("/etc/ssh"), &roots).is_empty());
    }

    #[test]
    fn navigation_is_refused_outside_your_files_and_drives() {
        assert!(resolve_navigable_path("/etc").is_err());
        assert!(resolve_navigable_path("/no/such/path").is_err());
    }

    #[test]
    fn folders_stay_first_in_both_sort_directions() {
        let listing = || vec![entry("big.iso", false, 900), entry("a.txt", false, 10), entry("Work", true, 0)];

        assert_eq!(sorted(listing(), EntrySort::Size, false), ["Work", "a.txt", "big.iso"]);
        assert_eq!(sorted(listing(), EntrySort::Size, true), ["Work", "big.iso", "a.txt"]);
        assert_eq!(sorted(listing(), EntrySort::Name, true), ["Work", "big.iso", "a.txt"]);
    }

    #[test]
    fn extensions_ignore_dotfiles_and_bare_names() {
        assert_eq!(entry_extension("notes.MD"), "md");
        assert_eq!(entry_extension("archive.tar.gz"), "gz");
        assert_eq!(entry_extension("README"), "");
        assert_eq!(entry_extension(".bashrc"), "");
    }

    #[test]
    fn names_cannot_escape_their_directory() {
        assert_eq!(validate_entry_name("  Reports ").ok(), Some("Reports"));
        assert!(validate_entry_name("").is_err());
        assert!(validate_entry_name("..").is_err());
        assert!(validate_entry_name(".").is_err());
        assert!(validate_entry_name("../etc").is_err());
        assert!(validate_entry_name("nested/name").is_err());
        assert!(validate_entry_name("back\\slash").is_err());
    }

    #[test]
    fn only_user_facing_linux_mounts_appear_as_drives() {
        assert!(is_user_visible_drive_mount(Path::new("/")));
        assert!(is_user_visible_drive_mount(Path::new(
            "/run/media/dave/USB"
        )));
        assert!(is_user_visible_drive_mount(Path::new("/media/USB")));
        assert!(!is_user_visible_drive_mount(Path::new("/proc")));
        assert!(!is_user_visible_drive_mount(Path::new("/var/lib/docker")));
        assert!(!is_user_visible_drive_mount(Path::new("/home")));
    }
}
