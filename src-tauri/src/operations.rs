use crate::error::DirectoryError;
use crate::paths::{resolve_navigable_path, vacant_target};
use std::{fs, path::Path};

pub(crate) fn copy_recursively(source: &Path, destination: &Path) -> std::io::Result<()> {
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

pub(crate) fn create_directory(parent_path: String, name: String) -> Result<String, DirectoryError> {
    let parent = resolve_navigable_path(&parent_path)?;

    if !parent.is_dir() {
        return Err(DirectoryError::unavailable());
    }

    let target = vacant_target(&parent, &name)?;
    fs::create_dir(&target).map_err(|_| DirectoryError::operation_failed())?;

    Ok(target.to_string_lossy().into_owned())
}

pub(crate) fn rename_entry(path: String, name: String) -> Result<String, DirectoryError> {
    let source = resolve_navigable_path(&path)?;
    let parent = source.parent().ok_or_else(DirectoryError::unavailable)?;
    let target = vacant_target(parent, &name)?;

    fs::rename(&source, &target).map_err(|_| DirectoryError::operation_failed())?;

    Ok(target.to_string_lossy().into_owned())
}

pub(crate) fn delete_entries(paths: Vec<String>) -> Result<(), DirectoryError> {
    let resolved = paths
        .iter()
        .map(|path| resolve_navigable_path(path))
        .collect::<Result<Vec<_>, _>>()?;

    trash::delete_all(&resolved).map_err(|_| DirectoryError::operation_failed())
}

pub(crate) fn transfer_entries(
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
