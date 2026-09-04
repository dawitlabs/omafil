use crate::drives::is_user_visible_drive_mount;
use crate::error::DirectoryError;
use std::path::{Path, PathBuf};
use sysinfo::Disks;

pub(crate) fn is_supported_location(location: &str) -> bool {
    matches!(
        location,
        "home" | "desktop" | "documents" | "downloads" | "pictures" | "videos" | "music"
    )
}

pub(crate) fn current_user_home_directory() -> Result<PathBuf, DirectoryError> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .ok_or_else(DirectoryError::unavailable)
}

pub(crate) fn known_directory_path(location: &str) -> Result<PathBuf, DirectoryError> {
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

pub(crate) fn navigable_roots() -> Vec<PathBuf> {
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

pub(crate) fn resolve_navigable_path(path: &str) -> Result<PathBuf, DirectoryError> {
    let requested = Path::new(path)
        .canonicalize()
        .map_err(|_| DirectoryError::unavailable())?;

    navigable_roots()
        .iter()
        .any(|root| requested.starts_with(root))
        .then_some(requested)
        .ok_or_else(DirectoryError::not_allowed)
}

pub(crate) fn display_name(directory: &Path) -> String {
    directory
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| directory.to_string_lossy().into_owned())
}

pub(crate) fn validate_entry_name(name: &str) -> Result<&str, DirectoryError> {
    let name = name.trim();
    let is_reserved = name.is_empty() || name == "." || name == "..";
    let has_separator = name.contains('/') || name.contains('\\') || name.contains('\0');

    if is_reserved || has_separator {
        return Err(DirectoryError::invalid_name());
    }

    Ok(name)
}

pub(crate) fn vacant_target(directory: &Path, name: &str) -> Result<PathBuf, DirectoryError> {
    let target = directory.join(validate_entry_name(name)?);

    if target.symlink_metadata().is_ok() {
        return Err(DirectoryError::already_exists());
    }

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::{is_supported_location, resolve_navigable_path, validate_entry_name};

    #[test]
    fn only_fixed_locations_are_allowed() {
        assert!(is_supported_location("home"));
        assert!(is_supported_location("desktop"));
        assert!(is_supported_location("music"));
        assert!(!is_supported_location("/etc"));
        assert!(!is_supported_location("../Desktop"));
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
    fn navigation_is_refused_outside_your_files_and_drives() {
        assert!(resolve_navigable_path("/etc").is_err());
        assert!(resolve_navigable_path("/no/such/path").is_err());
    }
}
