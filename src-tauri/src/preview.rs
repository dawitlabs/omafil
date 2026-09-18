use crate::error::DirectoryError;
use crate::paths::resolve_navigable_path;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    process::Command,
};

/// Only media consumed by passive img/audio/video elements can get an asset URL.
/// HTML, SVG, devices, sockets and pipes never receive a dynamic grant.
pub(crate) fn media_asset(path: &str) -> Result<PathBuf, DirectoryError> {
    let target = resolve_navigable_path(path)?;
    let extension = target.extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase();
    if !target.metadata()?.is_file() || !matches!(extension.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "bmp" |
        "mp3" | "wav" | "ogg" | "mp4" | "webm") {
        return Err(DirectoryError::detail("A media preview is not available for this item."));
    }
    // Check readability now so permission failures reach the UI rather than
    // appearing only as a failed asset request.
    fs::File::open(&target)?;
    Ok(target)
}

pub(crate) fn cache_root() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
}

fn cache_dir() -> Option<PathBuf> {
    Some(cache_root()?.join("omafil/pdf"))
}

/// First page of a PDF as a small PNG, rendered once per (path, mtime, size).
pub(crate) fn pdf_preview(path: String) -> Result<String, DirectoryError> {
    let target = resolve_navigable_path(&path)?;
    let metadata = target.metadata()?;
    if !metadata.is_file() || !target.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pdf")) {
        return Err(DirectoryError::detail("Only regular PDF files can be rendered."));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_home_media_is_supported_but_active_content_and_special_files_are_not() {
        let root = tempfile::tempdir().unwrap();
        let image = root.path().join("image.png");
        fs::write(&image, b"fixture").unwrap();
        assert_eq!(media_asset(image.to_str().unwrap()).unwrap(), image);
        for name in ["page.html", "image.svg", "folder.png"] {
            let path = root.path().join(name);
            if name == "folder.png" { fs::create_dir(&path).unwrap(); }
            else { fs::write(&path, b"fixture").unwrap(); }
            assert!(media_asset(path.to_str().unwrap()).is_err());
        }
        let socket = root.path().join("socket.png");
        let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        assert!(media_asset(socket.to_str().unwrap()).is_err());
    }
}
