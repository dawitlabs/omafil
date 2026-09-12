use crate::{error::DirectoryError, paths::resolve_navigable_path};
use serde::Serialize;
use std::{fs, path::Path, time::UNIX_EPOCH};

const MAX_TEXT_PREVIEW_BYTES: u64 = 48 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PathInspection {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) entry_type: String,
    pub(crate) size: u64,
    pub(crate) item_count: u64,
    pub(crate) modified: Option<i64>,
    pub(crate) created: Option<i64>,
    pub(crate) preview: Option<String>,
    pub(crate) preview_truncated: bool,
    pub(crate) media_preview: Option<String>,
    pub(crate) media_type: Option<String>,
}

fn media_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"), "jpg" | "jpeg" => Some("image/jpeg"), "gif" => Some("image/gif"), "webp" => Some("image/webp"),
        "pdf" => Some("application/pdf"), "mp3" => Some("audio/mpeg"), "wav" => Some("audio/wav"), "ogg" => Some("audio/ogg"),
        "mp4" => Some("video/mp4"), "webm" => Some("video/webm"), _ => None,
    }
}

fn timestamp(
    metadata: &fs::Metadata,
    getter: fn(&fs::Metadata) -> std::io::Result<std::time::SystemTime>,
) -> Option<i64> {
    getter(metadata)
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_secs() as i64)
}

fn text_preview(path: &Path, size: u64) -> (Option<String>, bool) {
    if size > 4 * 1024 * 1024 {
        return (None, false);
    }

    let Ok(bytes) = fs::read(path) else {
        return (None, false);
    };
    let shown = bytes.len().min(MAX_TEXT_PREVIEW_BYTES as usize);
    let Ok(text) = std::str::from_utf8(&bytes[..shown]) else {
        return (None, false);
    };

    // Control-heavy UTF-8 is not a helpful preview (for example, a binary
    // payload that happens to decode as UTF-8).
    if text
        .chars()
        .filter(|character| {
            character.is_control() && *character != '\n' && *character != '\r' && *character != '\t'
        })
        .count()
        > 4
    {
        return (None, false);
    }

    (Some(text.to_owned()), bytes.len() > shown)
}

fn folder_size(path: &Path) -> (u64, u64) {
    let mut total = 0;
    let mut count = 0;
    let Ok(entries) = fs::read_dir(path) else { return (0, 0) };
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if let Ok(metadata) = entry_path.symlink_metadata() {
            count += 1;
            if metadata.is_dir() { let (size, children) = folder_size(&entry_path); total += size; count += children; } else { total += metadata.len(); }
        }
    }
    (total, count)
}

pub(crate) fn inspect_path(path: String) -> Result<PathInspection, DirectoryError> {
    let target = resolve_navigable_path(&path)?;
    let metadata = target
        .metadata()
        .map_err(|_| DirectoryError::unavailable())?;
    let is_directory = metadata.is_dir();
    let (size, item_count) = if is_directory { folder_size(&target) } else { (metadata.len(), 1) };
    let (preview, preview_truncated) = (!is_directory)
        .then(|| text_preview(&target, metadata.len()))
        .unwrap_or((None, false));
    // Media itself is served through Tauri's scoped asset protocol, rather
    // than copying whole files into an IPC/Base64 response. That keeps large
    // photos and videos previewable without a size cap or UI memory spike.
    let media_type = (!is_directory).then(|| media_type(&target).map(str::to_owned)).flatten();

    Ok(PathInspection {
        name: target
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| target.to_string_lossy().into_owned()),
        path: target.to_string_lossy().into_owned(),
        entry_type: if is_directory { "folder" } else { "file" }.to_owned(),
        size,
        item_count,
        modified: timestamp(&metadata, fs::Metadata::modified),
        created: timestamp(&metadata, fs::Metadata::created),
        preview,
        preview_truncated,
        media_preview: None,
        media_type,
    })
}
