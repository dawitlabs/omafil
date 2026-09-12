use crate::error::DirectoryError;
use crate::operation_io::{measure, OperationContext, StagedOutput};
use crate::paths::{resolve_entry_path, resolve_navigable_path, vacant_target};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

fn zip_error(error: zip::result::ZipError) -> DirectoryError {
    DirectoryError::detail(format!("Unable to process this ZIP archive: {error}"))
}

fn add_path(
    writer: &mut ZipWriter<fs::File>,
    path: &Path,
    base: &Path,
    context: &mut OperationContext<'_>,
) -> Result<(), DirectoryError> {
    context.check()?;
    context.current(path);
    let metadata = path.symlink_metadata()?;
    // Never follow a link into an unrelated tree (or a cycle).
    if metadata.is_symlink() {
        return Err(DirectoryError::detail(
            "ZIP creation does not support symbolic links. No archive was saved.",
        ));
    }
    let relative = path
        .strip_prefix(base)
        .map_err(|_| DirectoryError::invalid_destination())?;
    let name = relative
        .to_str()
        .ok_or_else(|| DirectoryError::detail("ZIP filenames must be valid UTF-8."))?;
    if name.contains('\\') {
        return Err(DirectoryError::detail(
            "ZIP filenames cannot contain backslashes.",
        ));
    }
    if metadata.is_dir() {
        writer
            .add_directory(format!("{name}/"), SimpleFileOptions::default())
            .map_err(zip_error)?;
        for entry in fs::read_dir(path)? {
            add_path(writer, &entry?.path(), base, context)?;
        }
    } else if metadata.is_file() {
        writer
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .map_err(zip_error)?;
        context.copy(&mut fs::File::open(path)?, writer)?;
    } else {
        return Err(DirectoryError::detail(
            "Sockets, devices and named pipes cannot be archived.",
        ));
    }
    Ok(())
}

pub(crate) fn create_zip(
    paths: Vec<String>,
    destination_path: String,
    name: String,
) -> Result<String, DirectoryError> {
    let cancel = AtomicBool::new(false);
    let mut context = OperationContext::new(&cancel, |_| {});
    create_zip_with_context(paths, destination_path, name, &mut context)
}

pub(crate) fn create_zip_with_context(
    paths: Vec<String>,
    destination_path: String,
    name: String,
    context: &mut OperationContext<'_>,
) -> Result<String, DirectoryError> {
    let destination = resolve_navigable_path(&destination_path)?;
    let sources = paths
        .iter()
        .map(|path| resolve_entry_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    create_at(&sources, &destination, &name, context)
        .map(|path| path.to_string_lossy().into_owned())
}

fn create_at(
    sources: &[PathBuf],
    destination: &Path,
    name: &str,
    context: &mut OperationContext<'_>,
) -> Result<PathBuf, DirectoryError> {
    context.check()?;
    if sources.is_empty() {
        return Err(DirectoryError::detail(
            "Select at least one item to archive.",
        ));
    }
    let name = if name.to_lowercase().ends_with(".zip") {
        name.to_owned()
    } else {
        format!("{name}.zip")
    };
    let target = vacant_target(destination, &name)?;
    let mut total = 0u64;
    for source in sources {
        if destination.starts_with(source) {
            return Err(DirectoryError::invalid_destination());
        }
        total = total.saturating_add(measure(source, context)?);
    }
    context.set_total(total);
    let stage = StagedOutput::new(destination)?;
    let mut writer = ZipWriter::new(fs::File::create_new(&stage.path)?);
    for source in sources {
        let base = source.parent().ok_or_else(DirectoryError::unavailable)?;
        add_path(&mut writer, source, base, context)?;
    }
    context.check()?;
    writer.finish().map_err(zip_error)?.sync_all()?;
    stage.publish(&target, false, context)?;
    context.current(&target);
    context.complete_item();
    Ok(target)
}

pub(crate) fn extract_zip(path: String, destination_path: String) -> Result<(), DirectoryError> {
    let cancel = AtomicBool::new(false);
    let mut context = OperationContext::new(&cancel, |_| {});
    extract_zip_with_context(path, destination_path, &mut context)
}

pub(crate) fn extract_zip_with_context(
    path: String,
    destination_path: String,
    context: &mut OperationContext<'_>,
) -> Result<(), DirectoryError> {
    let archive_path = resolve_navigable_path(&path)?;
    let destination = resolve_navigable_path(&destination_path)?;
    extract_at(&archive_path, &destination, context).map(|_| ())
}

fn extraction_target(destination: &Path, archive_path: &Path) -> Result<PathBuf, DirectoryError> {
    let stem = archive_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Archive");
    let stem = if stem.is_empty() || stem == "." || stem == ".." {
        "Archive"
    } else {
        stem
    };
    for suffix in 1..10_000 {
        let name = if suffix == 1 {
            stem.to_owned()
        } else {
            format!("{stem} ({suffix})")
        };
        let target = destination.join(name);
        match target.symlink_metadata() {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(target),
            Err(error) => return Err(error.into()),
            Ok(_) => {}
        }
    }
    Err(DirectoryError::already_exists())
}

fn extract_at(
    archive_path: &Path,
    destination: &Path,
    context: &mut OperationContext<'_>,
) -> Result<PathBuf, DirectoryError> {
    context.check()?;
    let target = extraction_target(destination, archive_path)?;
    let mut archive = ZipArchive::new(fs::File::open(archive_path)?).map_err(zip_error)?;
    let mut total = 0u64;
    for index in 0..archive.len() {
        context.check()?;
        let entry = archive.by_index(index).map_err(zip_error)?;
        total = total.saturating_add(entry.size());
    }
    context.set_total(total);
    let stage = StagedOutput::new(destination)?;
    fs::create_dir(&stage.path)?;
    for index in 0..archive.len() {
        context.check()?;
        let mut entry = archive.by_index(index).map_err(zip_error)?;
        let name = entry.enclosed_name().ok_or_else(|| {
            DirectoryError::detail("This ZIP contains a path outside its extraction folder.")
        })?;
        if entry.name().contains('\\') || name.as_os_str().is_empty() {
            return Err(DirectoryError::detail(
                "This ZIP contains an unsupported entry name.",
            ));
        }
        let kind = entry.unix_mode().unwrap_or(0) & 0o170000;
        if kind != 0 && kind != 0o100000 && kind != 0o040000 {
            return Err(DirectoryError::detail(
                "ZIP extraction does not support symbolic links or special files.",
            ));
        }
        let output = stage.path.join(name);
        context.current(&output);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
        } else {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            // Duplicate entries cannot overwrite earlier output, either.
            let mut file = fs::File::create_new(&output)?;
            context.copy(&mut entry, &mut file)?;
            file.sync_all()?;
        }
    }
    stage.publish(&target, false, context)?;
    context.current(&target);
    context.complete_item();
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, sync::atomic::Ordering};

    #[test]
    fn zip_round_trip_uses_a_new_folder_and_preserves_existing_files() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("note.txt");
        fs::write(&source, b"hello archive").unwrap();
        let cancel = AtomicBool::new(false);
        let mut context = OperationContext::new(&cancel, |_| {});
        let archive = create_at(&[source.clone()], root.path(), "bundle", &mut context).unwrap();
        fs::create_dir(root.path().join("bundle")).unwrap();
        fs::write(root.path().join("bundle/note.txt"), b"keep me").unwrap();
        let extracted = extract_at(
            &archive,
            root.path(),
            &mut OperationContext::new(&cancel, |_| {}),
        )
        .unwrap();
        assert_eq!(extracted, root.path().join("bundle (2)"));
        assert_eq!(
            fs::read(extracted.join("note.txt")).unwrap(),
            b"hello archive"
        );
        assert_eq!(
            fs::read(root.path().join("bundle/note.txt")).unwrap(),
            b"keep me"
        );
        assert!(create_at(&[source], root.path(), "bundle.zip", &mut context).is_err());
    }

    #[test]
    fn cancelled_archive_creation_leaves_no_partial_archive() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("large");
        fs::write(&source, vec![9; 1024 * 1024]).unwrap();
        let cancel = AtomicBool::new(false);
        let mut context = OperationContext::new(&cancel, |progress| {
            if progress.completed_bytes > 0 {
                cancel.store(true, Ordering::Relaxed);
            }
        });
        assert!(create_at(&[source], root.path(), "bundle", &mut context)
            .err()
            .unwrap()
            .is_cancelled());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn cancelled_extraction_leaves_no_partial_folder() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("large");
        fs::write(&source, vec![9; 1024 * 1024]).unwrap();
        let cancel = AtomicBool::new(false);
        let archive = create_at(
            &[source],
            root.path(),
            "bundle",
            &mut OperationContext::new(&cancel, |_| {}),
        )
        .unwrap();
        let mut context = OperationContext::new(&cancel, |progress| {
            if progress.completed_bytes > 0 {
                cancel.store(true, Ordering::Relaxed);
            }
        });
        assert!(extract_at(&archive, root.path(), &mut context)
            .err()
            .unwrap()
            .is_cancelled());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[test]
    fn traversal_archive_is_rejected_without_publishing_output() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("unsafe.zip");
        let mut writer = ZipWriter::new(fs::File::create(&archive).unwrap());
        writer
            .start_file("../escaped", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"bad").unwrap();
        writer.finish().unwrap();
        let cancel = AtomicBool::new(false);
        assert!(extract_at(
            &archive,
            root.path(),
            &mut OperationContext::new(&cancel, |_| {})
        )
        .is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn symlink_archive_is_rejected_without_following_its_target() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("unsafe.zip");
        let mut writer = ZipWriter::new(fs::File::create(&archive).unwrap());
        writer
            .add_symlink("link", "../outside", SimpleFileOptions::default())
            .unwrap();
        writer.finish().unwrap();
        let cancel = AtomicBool::new(false);
        assert!(extract_at(
            &archive,
            root.path(),
            &mut OperationContext::new(&cancel, |_| {})
        )
        .is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn archive_cannot_include_its_own_output_directory() {
        let root = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        assert!(create_at(
            &[root.path().to_path_buf()],
            root.path(),
            "recursive",
            &mut OperationContext::new(&cancel, |_| {})
        )
        .is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn symbolic_link_cycles_are_rejected_when_archiving() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("folder");
        fs::create_dir(&source).unwrap();
        std::os::unix::fs::symlink(".", source.join("loop")).unwrap();
        let cancel = AtomicBool::new(false);
        assert!(create_at(
            &[source],
            root.path(),
            "bundle",
            &mut OperationContext::new(&cancel, |_| {})
        )
        .is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
