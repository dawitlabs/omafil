use crate::error::DirectoryError;
use crate::operation_io::{
    copy_tree, measure, rename_without_replace, OperationContext, StagedOutput,
};
use crate::paths::{resolve_entry_path, resolve_navigable_path, vacant_target};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferConflictPolicy {
    Fail,
    Skip,
    Replace,
    Rename,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferConflict {
    pub source_path: String,
    pub destination_path: String,
    pub name: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferResult {
    pub source_path: String,
    pub destination_path: String,
    pub skipped: bool,
}

pub fn create_directory(
    parent_path: String,
    name: String,
) -> Result<String, DirectoryError> {
    let parent = resolve_navigable_path(&parent_path)?;

    if !parent.is_dir() {
        return Err(DirectoryError::unavailable());
    }

    let target = vacant_target(&parent, &name)?;
    fs::create_dir(&target).map_err(DirectoryError::from)?;

    Ok(target.to_string_lossy().into_owned())
}

pub fn rename_entry(path: String, name: String) -> Result<String, DirectoryError> {
    let source = resolve_entry_path(&path)?;
    let parent = source.parent().ok_or_else(DirectoryError::unavailable)?;
    let target = vacant_target(parent, &name)?;

    rename_without_replace(&source, &target).map_err(DirectoryError::from)?;

    Ok(target.to_string_lossy().into_owned())
}

/// Returns the trash ids of what it removed, which is what restoring them later needs.
pub fn delete_entries(paths: Vec<String>) -> Result<Vec<String>, DirectoryError> {
    let resolved = paths
        .iter()
        .map(|path| resolve_entry_path(path))
        .collect::<Result<Vec<_>, _>>()?;

    let known = crate::recycle::recycle_item_ids()?;
    trash::delete_all(&resolved).map_err(|_| DirectoryError::operation_failed())?;

    Ok(crate::recycle::recycle_item_ids()?
        .into_iter()
        .filter(|id| !known.contains(id))
        .collect())
}

pub fn transfer_entries(
    paths: Vec<String>,
    destination_path: String,
    is_move: bool,
    conflict_policy: TransferConflictPolicy,
) -> Result<Vec<TransferResult>, DirectoryError> {
    let cancellation = AtomicBool::new(false);
    let mut results = Vec::new();
    transfer_with_context(
        paths,
        destination_path,
        is_move,
        conflict_policy,
        &mut OperationContext::new(&cancellation, |_| {}),
        &mut results,
    )?;
    Ok(results)
}

/// Completed results survive a later failure/cancellation and are sent to the UI.
pub fn transfer_with_context(
    paths: Vec<String>,
    destination_path: String,
    is_move: bool,
    policy: TransferConflictPolicy,
    context: &mut OperationContext<'_>,
    results: &mut Vec<TransferResult>,
) -> Result<(), DirectoryError> {
    let destination = resolve_navigable_path(&destination_path)?;
    if !destination.is_dir() {
        return Err(DirectoryError::unavailable());
    }
    let mut sources = Vec::with_capacity(paths.len());
    let mut total = 0u64;
    for path in paths {
        context.check()?;
        let source = resolve_entry_path(&path)?;
        let bytes = measure(&source, context)?;
        total = total.saturating_add(bytes);
        sources.push((source, bytes));
    }
    context.set_total(total);
    for (source, bytes) in sources {
        results.push(transfer_one(
            &source,
            &destination,
            is_move,
            policy,
            bytes,
            context,
        )?);
        context.complete_item();
    }
    Ok(())
}

fn transfer_one(
    source: &Path,
    destination: &Path,
    is_move: bool,
    policy: TransferConflictPolicy,
    bytes: u64,
    context: &mut OperationContext<'_>,
) -> Result<TransferResult, DirectoryError> {
    context.check()?;
    context.current(source);
    let name = source.file_name().ok_or_else(DirectoryError::unavailable)?;
    let mut target = destination.join(name);
    if destination.starts_with(source) || source.starts_with(&target) {
        return Err(DirectoryError::invalid_destination());
    }
    let mut replace = false;
    if target.symlink_metadata().is_ok() {
        match policy {
            TransferConflictPolicy::Fail => return Err(DirectoryError::already_exists()),
            TransferConflictPolicy::Skip => {
                context.advance(bytes);
                return Ok(TransferResult {
                    source_path: source.to_string_lossy().into_owned(),
                    destination_path: target.to_string_lossy().into_owned(),
                    skipped: true,
                });
            }
            TransferConflictPolicy::Replace => replace = true,
            TransferConflictPolicy::Rename => {
                target = renamed_target(destination, &name.to_string_lossy())?
            }
        }
    }
    context.check()?;
    if is_move && !replace {
        match rename_without_replace(source, &target) {
            Ok(()) => {
                context.advance(bytes);
                return Ok(TransferResult {
                    source_path: source.to_string_lossy().into_owned(),
                    destination_path: target.to_string_lossy().into_owned(),
                    skipped: false,
                });
            }
            Err(error) if error.raw_os_error() == Some(rustix::io::Errno::XDEV.raw_os_error()) => {}
            Err(error) => return Err(error.into()),
        }
    }
    let stage = StagedOutput::new(destination)?;
    copy_tree(source, &stage.path, context)?;
    stage.publish(&target, replace, context)?;
    // Publication is the commit point. Finish source removal even if Cancel arrives now.
    if is_move {
        trash::delete(source).map_err(|error| DirectoryError::detail(format!(
            "Copied to {}, but the source could not be removed: {error}. Both copies were kept.", target.display()
        )))?;
    }
    Ok(TransferResult {
        source_path: source.to_string_lossy().into_owned(),
        destination_path: target.to_string_lossy().into_owned(),
        skipped: false,
    })
}

fn renamed_target(destination: &Path, name: &str) -> Result<PathBuf, DirectoryError> {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    let extension = path.extension().and_then(|value| value.to_str());

    for suffix in 2..10_000 {
        let candidate_name = match extension {
            Some(extension) => format!("{stem} ({suffix}).{extension}"),
            None => format!("{stem} ({suffix})"),
        };
        let candidate = destination.join(candidate_name);
        if candidate.symlink_metadata().is_err() {
            return Ok(candidate);
        }
    }

    Err(DirectoryError::operation_failed())
}

pub fn permanently_delete_entries(paths: Vec<String>) -> Result<(), DirectoryError> {
    for path in paths {
        let target = resolve_entry_path(&path)?;
        let metadata = target
            .symlink_metadata()
            .map_err(DirectoryError::from)?;

        if metadata.is_dir() {
            fs::remove_dir_all(target).map_err(DirectoryError::from)?;
        } else {
            fs::remove_file(target).map_err(DirectoryError::from)?;
        }
    }

    Ok(())
}

pub fn find_transfer_conflicts(
    paths: Vec<String>,
    destination_path: String,
) -> Result<Vec<TransferConflict>, DirectoryError> {
    let destination = resolve_navigable_path(&destination_path)?;

    if !destination.is_dir() {
        return Err(DirectoryError::unavailable());
    }

    paths
        .into_iter()
        .map(|path| {
            let source = resolve_entry_path(&path)?;
            let name = source
                .file_name()
                .ok_or_else(DirectoryError::unavailable)?
                .to_string_lossy()
                .into_owned();
            let target: PathBuf = destination.join(&name);

            Ok(target.symlink_metadata().is_ok().then(|| TransferConflict {
                source_path: source.to_string_lossy().into_owned(),
                destination_path: target.to_string_lossy().into_owned(),
                name,
            }))
        })
        .filter_map(Result::transpose)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn deleting_a_symlink_leaves_its_target_intact() {
        let home = crate::paths::current_user_home_directory().unwrap();
        let root = tempfile::Builder::new()
            .prefix(".omafil-test-")
            .tempdir_in(home)
            .unwrap();
        let target = root.path().join("document");
        let link = root.path().join("shortcut");
        fs::write(&target, b"keep me").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        permanently_delete_entries(vec![link.to_string_lossy().into_owned()]).unwrap();
        assert!(link.symlink_metadata().is_err());
        assert_eq!(fs::read(target).unwrap(), b"keep me");
    }

    #[test]
    fn cancelled_replacement_keeps_original_and_removes_staging() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("document");
        let destination = root.path().join("destination");
        fs::create_dir(&destination).unwrap();
        fs::write(&source, vec![7; 1024 * 1024]).unwrap();
        fs::write(destination.join("document"), b"original").unwrap();
        let cancel = AtomicBool::new(false);
        let mut context = OperationContext::new(&cancel, |progress| {
            if progress.completed_bytes > 0 {
                cancel.store(true, Ordering::Relaxed);
            }
        });
        let result = transfer_one(
            &source,
            &destination,
            true,
            TransferConflictPolicy::Replace,
            1024 * 1024,
            &mut context,
        );
        assert!(result.err().unwrap().is_cancelled());
        assert_eq!(fs::read(destination.join("document")).unwrap(), b"original");
        assert_eq!(fs::metadata(source).unwrap().len(), 1024 * 1024);
        assert_eq!(fs::read_dir(destination).unwrap().count(), 1);
    }

    #[test]
    fn copies_nested_files_and_preserves_dangling_symlinks() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("folder");
        let destination = root.path().join("destination");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir(&destination).unwrap();
        fs::write(source.join("nested/file"), b"hello").unwrap();
        std::os::unix::fs::symlink("missing", source.join("link")).unwrap();
        let cancel = AtomicBool::new(false);
        let mut context = OperationContext::new(&cancel, |_| {});
        assert!(transfer_one(
            &source,
            &destination,
            false,
            TransferConflictPolicy::Fail,
            5,
            &mut context
        )
        .is_ok());
        assert_eq!(
            fs::read(destination.join("folder/nested/file")).unwrap(),
            b"hello"
        );
        assert_eq!(
            fs::read_link(destination.join("folder/link")).unwrap(),
            PathBuf::from("missing")
        );
        assert_eq!(context.progress.completed_bytes, 5);
    }

    #[test]
    fn keep_both_and_skip_preserve_existing_data() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("report.txt");
        let destination = root.path().join("destination");
        fs::create_dir(&destination).unwrap();
        fs::write(&source, b"new").unwrap();
        fs::write(destination.join("report.txt"), b"old").unwrap();
        let cancel = AtomicBool::new(false);
        let mut context = OperationContext::new(&cancel, |_| {});
        let skipped = transfer_one(
            &source,
            &destination,
            true,
            TransferConflictPolicy::Skip,
            3,
            &mut context,
        )
        .unwrap();
        assert!(skipped.skipped);
        assert!(source.exists());
        assert!(transfer_one(
            &source,
            &destination,
            false,
            TransferConflictPolicy::Rename,
            3,
            &mut context
        )
        .is_ok());
        assert_eq!(fs::read(destination.join("report.txt")).unwrap(), b"old");
        assert_eq!(
            fs::read(destination.join("report (2).txt")).unwrap(),
            b"new"
        );
    }

    #[test]
    fn same_volume_move_publishes_and_removes_source() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("file");
        let destination = root.path().join("destination");
        fs::create_dir(&destination).unwrap();
        fs::write(&source, b"moved").unwrap();
        let cancel = AtomicBool::new(false);
        assert!(transfer_one(
            &source,
            &destination,
            true,
            TransferConflictPolicy::Fail,
            5,
            &mut OperationContext::new(&cancel, |_| {})
        )
        .is_ok());
        assert!(!source.exists());
        assert_eq!(fs::read(destination.join("file")).unwrap(), b"moved");
    }
}
