use crate::error::DirectoryError;
use std::path::{Path, PathBuf};

pub fn is_supported_location(location: &str) -> bool {
    matches!(
        location,
        "home" | "desktop" | "documents" | "downloads" | "pictures" | "videos" | "music"
    )
}

pub fn current_user_home_directory() -> Result<PathBuf, DirectoryError> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .ok_or_else(DirectoryError::unavailable)
}

pub fn known_directory_path(location: &str) -> Result<PathBuf, DirectoryError> {
    if !is_supported_location(location) {
        return Err(DirectoryError::unavailable());
    }

    let home_directory = current_user_home_directory()?;
    if location == "home" {
        return home_directory.canonicalize().map_err(DirectoryError::from);
    }
    crate::user_dirs::resolve(location, &home_directory)
}

pub fn navigable_roots() -> Vec<PathBuf> {
    vec![PathBuf::from("/")]
}

/// Normal OS permissions govern access, including outside home and mounted drives.
/// Entry mutations use `resolve_entry_path` to preserve the final symlink.
pub fn resolve_navigable_path(path: &str) -> Result<PathBuf, DirectoryError> {
    Path::new(path).canonicalize().map_err(DirectoryError::from)
}

/// Resolve the parent, preserving the final entry (including dangling symlinks).
pub fn resolve_entry_path(path: &str) -> Result<PathBuf, DirectoryError> {
    let path = Path::new(path);
    let name = path.file_name().ok_or_else(DirectoryError::unavailable)?;
    let parent = path.parent().ok_or_else(DirectoryError::unavailable)?;
    let parent = resolve_navigable_path(&parent.to_string_lossy())?;
    let entry = parent.join(name);
    entry
        .symlink_metadata()
        .map_err(DirectoryError::from)?;
    Ok(entry)
}

pub fn display_name(directory: &Path) -> String {
    directory
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| directory.to_string_lossy().into_owned())
}

pub fn validate_entry_name(name: &str) -> Result<&str, DirectoryError> {
    let name = name.trim();
    let is_reserved = name.is_empty() || name == "." || name == "..";
    let has_separator = name.contains('/') || name.contains('\\') || name.contains('\0');

    if is_reserved || has_separator {
        return Err(DirectoryError::invalid_name());
    }

    Ok(name)
}

pub fn vacant_target(directory: &Path, name: &str) -> Result<PathBuf, DirectoryError> {
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
    fn navigation_uses_normal_filesystem_permissions() {
        assert!(resolve_navigable_path("/etc").is_ok());
        let directory = tempfile::tempdir().unwrap();
        assert!(resolve_navigable_path(directory.path().to_str().unwrap()).is_ok());
        assert!(resolve_navigable_path("/no/such/path").is_err());
    }

    #[test]
    fn entry_operations_refuse_root_but_preserve_final_symlinks() {
        use super::resolve_entry_path;
        assert!(resolve_entry_path("/").is_err());
        assert!(resolve_entry_path("/tmp/..").is_err());
        let root = tempfile::tempdir().unwrap();
        let link = root.path().join("root-link");
        std::os::unix::fs::symlink("/", &link).unwrap();
        assert_eq!(resolve_entry_path(link.to_str().unwrap()).unwrap(), link);
        assert_eq!(resolve_navigable_path(link.to_str().unwrap()).unwrap(), std::path::PathBuf::from("/"));
    }
}
