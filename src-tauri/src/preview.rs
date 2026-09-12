use crate::error::DirectoryError;
use crate::paths::resolve_navigable_path;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    process::Command,
};

fn cache_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;

    Some(base.join("omafil/pdf"))
}

/// First page of a PDF as a small PNG, rendered once per (path, mtime, size).
pub(crate) fn pdf_preview(path: String) -> Result<String, DirectoryError> {
    let target = resolve_navigable_path(&path)?;
    let metadata = target.metadata()?;
    let mut hasher = DefaultHasher::new();
    target.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    metadata.modified().ok().hash(&mut hasher);
    let dir = cache_dir().ok_or_else(|| DirectoryError::detail("No cache directory is available."))?;
    let stem = dir.join(format!("{:016x}", hasher.finish()));
    let png = stem.with_extension("png");

    if png.is_file() {
        return Ok(png.to_string_lossy().into_owned());
    }
    fs::create_dir_all(&dir)?;

    let rendered = Command::new("pdftoppm")
        .args(["-png", "-r", "48", "-f", "1", "-l", "1", "-singlefile"])
        .arg(&target)
        .arg(&stem)
        .output()
        .map_err(|_| DirectoryError::detail("pdftoppm (poppler) is not installed."))?;

    if !rendered.status.success() || !png.is_file() {
        return Err(DirectoryError::detail("Unable to render this PDF."));
    }

    Ok(png.to_string_lossy().into_owned())
}
