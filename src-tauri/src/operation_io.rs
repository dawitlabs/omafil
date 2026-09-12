//! Shared cancellation, byte accounting and publication for filesystem jobs.
use crate::error::DirectoryError;
use rustix::fs::{renameat_with, RenameFlags, CWD};
use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

const CHUNK_SIZE: usize = 256 * 1024;

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OperationProgress {
    pub(crate) completed_bytes: u64,
    pub(crate) completed_items: usize,
    pub(crate) total_bytes: Option<u64>,
    pub(crate) current_name: Option<String>,
}

pub(crate) struct OperationContext<'a> {
    cancellation: &'a AtomicBool,
    notify: Box<dyn FnMut(&OperationProgress) + 'a>,
    pub(crate) progress: OperationProgress,
    last_emit: Instant,
}

impl<'a> OperationContext<'a> {
    pub(crate) fn new(
        cancellation: &'a AtomicBool,
        notify: impl FnMut(&OperationProgress) + 'a,
    ) -> Self {
        Self {
            cancellation,
            notify: Box::new(notify),
            progress: OperationProgress::default(),
            last_emit: Instant::now(),
        }
    }

    pub(crate) fn check(&self) -> Result<(), DirectoryError> {
        if self.cancellation.load(Ordering::Relaxed) {
            Err(DirectoryError::cancelled())
        } else {
            Ok(())
        }
    }

    pub(crate) fn complete_item(&mut self) {
        self.progress.completed_items += 1;
        if self.last_emit.elapsed() >= Duration::from_millis(100) {
            self.emit();
        }
    }

    pub(crate) fn set_total(&mut self, total: u64) {
        self.progress.total_bytes = Some(total);
        self.emit();
    }

    pub(crate) fn current(&mut self, path: &Path) {
        self.progress.current_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        if self.last_emit.elapsed() >= Duration::from_millis(100) {
            self.emit();
        }
    }

    fn emit(&mut self) {
        (self.notify)(&self.progress);
        self.last_emit = Instant::now();
    }

    pub(crate) fn advance(&mut self, bytes: u64) {
        let first = bytes > 0 && self.progress.completed_bytes == 0;
        self.progress.completed_bytes = self.progress.completed_bytes.saturating_add(bytes);
        if let Some(total) = &mut self.progress.total_bytes {
            *total = (*total).max(self.progress.completed_bytes);
        }
        if first || self.last_emit.elapsed() >= Duration::from_millis(100) {
            self.emit();
        }
    }

    pub(crate) fn copy(
        &mut self,
        reader: &mut impl Read,
        writer: &mut impl Write,
    ) -> Result<(), DirectoryError> {
        let mut buffer = vec![0; CHUNK_SIZE];
        loop {
            self.check()?;
            let count = match reader.read(&mut buffer) {
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                result => result?,
            };
            if count == 0 {
                break;
            }
            self.check()?;
            writer.write_all(&buffer[..count])?;
            self.advance(count as u64);
        }
        self.check()
    }
}

pub(crate) fn measure(path: &Path, context: &OperationContext<'_>) -> Result<u64, DirectoryError> {
    context.check()?;
    let metadata = path.symlink_metadata()?;
    if metadata.is_symlink() {
        return Ok(0);
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err(DirectoryError::detail(
            "Sockets, devices and named pipes cannot be copied.",
        ));
    }
    let mut bytes: u64 = 0;
    for entry in fs::read_dir(path)? {
        bytes = bytes.saturating_add(measure(&entry?.path(), context)?);
    }
    Ok(bytes)
}

pub(crate) fn copy_tree(
    source: &Path,
    target: &Path,
    context: &mut OperationContext<'_>,
) -> Result<(), DirectoryError> {
    context.check()?;
    let metadata = source.symlink_metadata()?;
    context.current(source);
    if metadata.is_symlink() {
        std::os::unix::fs::symlink(fs::read_link(source)?, target)?;
    } else if metadata.is_dir() {
        fs::create_dir(target)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &target.join(entry.file_name()), context)?;
        }
        fs::set_permissions(target, metadata.permissions())?;
    } else if metadata.is_file() {
        let mut input = fs::File::open(source)?;
        let mut output = fs::File::create_new(target)?;
        context.copy(&mut input, &mut output)?;
        output.sync_all()?;
        fs::set_permissions(target, metadata.permissions())?;
    } else {
        return Err(DirectoryError::detail(
            "Sockets, devices and named pipes cannot be copied.",
        ));
    }
    context.check()
}

/// Linux atomic publication: never overwrite an item that appeared while copying.
pub(crate) fn rename_without_replace(source: &Path, target: &Path) -> std::io::Result<()> {
    renameat_with(CWD, source, CWD, target, RenameFlags::NOREPLACE).map_err(Into::into)
}

pub(crate) struct StagedOutput {
    directory: tempfile::TempDir,
    pub(crate) path: PathBuf,
}

impl StagedOutput {
    pub(crate) fn new(parent: &Path) -> Result<Self, DirectoryError> {
        let directory = tempfile::Builder::new()
            .prefix(".omafil-operation-")
            .tempdir_in(parent)?;
        let path = directory.path().join("content");
        Ok(Self { directory, path })
    }

    pub(crate) fn publish(
        self,
        target: &Path,
        replace: bool,
        context: &mut OperationContext<'_>,
    ) -> Result<(), DirectoryError> {
        self.publish_with_trash(target, replace, context, |path| {
            trash::delete(path).map_err(|error| {
                DirectoryError::detail(format!(
                    "Unable to preserve the previous item in Trash: {error}"
                ))
            })
        })
    }

    fn publish_with_trash(
        self,
        target: &Path,
        replace: bool,
        context: &mut OperationContext<'_>,
        move_to_trash: impl FnOnce(&Path) -> Result<(), DirectoryError>,
    ) -> Result<(), DirectoryError> {
        context.check()?;
        let replaced = replace && target.symlink_metadata().is_ok();
        if replaced {
            // Preserve the original name and restore location in the desktop Trash.
            // This starts the commit phase: do not honour cancellation halfway through it.
            move_to_trash(target)?;
        }
        rename_without_replace(&self.path, target).map_err(|error| {
            if replaced {
                DirectoryError::detail(format!("Unable to save the replacement: {error}. The previous item is in Trash and the source was kept."))
            } else { error.into() }
        })?;
        Ok(())
    }
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        // A copied read-only directory must not strand an interrupted staging tree.
        // Only adjust directories inside our private staging area; never follow links.
        fn make_removable(path: &Path) {
            use std::os::unix::fs::PermissionsExt;
            let Ok(metadata) = path.symlink_metadata() else {
                return;
            };
            if !metadata.is_dir() {
                return;
            }
            let _ = fs::set_permissions(
                path,
                fs::Permissions::from_mode(metadata.permissions().mode() | 0o700),
            );
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    make_removable(&entry.path());
                }
            }
        }
        make_removable(self.directory.path());
        // TempDir then removes this tree as its field is dropped.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_read_only_tree_is_cleaned_up_without_following_links() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let stage = StagedOutput::new(root.path()).unwrap();
        fs::create_dir(&stage.path).unwrap();
        fs::write(stage.path.join("file"), b"partial").unwrap();
        std::os::unix::fs::symlink(outside.path(), stage.path.join("link")).unwrap();
        fs::set_permissions(&stage.path, fs::Permissions::from_mode(0o500)).unwrap();
        let outside_mode = fs::metadata(outside.path()).unwrap().permissions().mode();
        drop(stage);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        assert_eq!(
            fs::metadata(outside.path()).unwrap().permissions().mode(),
            outside_mode
        );
    }

    #[test]
    fn cancellation_interrupts_a_single_large_stream() {
        let cancellation = AtomicBool::new(false);
        let mut context = OperationContext::new(&cancellation, |progress| {
            if progress.completed_bytes > 0 {
                cancellation.store(true, Ordering::Relaxed);
            }
        });
        let data = vec![42; CHUNK_SIZE * 4];
        let mut output = Vec::new();
        let error = context.copy(&mut data.as_slice(), &mut output).unwrap_err();
        assert!(error.is_cancelled());
        assert_eq!(output.len(), CHUNK_SIZE);
    }

    #[test]
    fn replacement_preserves_the_original_when_publication_fails() {
        let root = tempfile::tempdir().unwrap();
        let stage = StagedOutput::new(root.path()).unwrap();
        fs::write(&stage.path, b"new").unwrap();
        let target = root.path().join("document");
        let recovery = root.path().join("trash-document");
        fs::write(&target, b"old").unwrap();
        let cancel = AtomicBool::new(false);
        let error = stage
            .publish_with_trash(
                &target,
                true,
                &mut OperationContext::new(&cancel, |_| {}),
                |path| {
                    assert_eq!(path, target);
                    fs::rename(path, &recovery)?;
                    fs::write(path, b"late arrival")?;
                    Ok(())
                },
            )
            .unwrap_err();
        assert!(error.message().contains("previous item is in Trash"));
        assert_eq!(fs::read(recovery).unwrap(), b"old");
        assert_eq!(fs::read(target).unwrap(), b"late arrival");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[test]
    fn replacement_finishes_commit_if_cancel_arrives_after_trashing() {
        let root = tempfile::tempdir().unwrap();
        let stage = StagedOutput::new(root.path()).unwrap();
        fs::write(&stage.path, b"new").unwrap();
        let target = root.path().join("document");
        let recovery = root.path().join("trash-document");
        fs::write(&target, b"old").unwrap();
        let cancel = AtomicBool::new(false);
        stage
            .publish_with_trash(
                &target,
                true,
                &mut OperationContext::new(&cancel, |_| {}),
                |path| {
                    fs::rename(path, &recovery)?;
                    cancel.store(true, Ordering::Relaxed);
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(fs::read(recovery).unwrap(), b"old");
        assert_eq!(fs::read(target).unwrap(), b"new");
    }

    #[test]
    fn publication_does_not_overwrite_a_late_arrival() {
        let root = tempfile::tempdir().unwrap();
        let stage = StagedOutput::new(root.path()).unwrap();
        fs::write(&stage.path, b"new").unwrap();
        let target = root.path().join("document");
        fs::write(&target, b"existing").unwrap();
        let cancel = AtomicBool::new(false);
        assert!(stage
            .publish(&target, false, &mut OperationContext::new(&cancel, |_| {}))
            .is_err());
        assert_eq!(fs::read(target).unwrap(), b"existing");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
