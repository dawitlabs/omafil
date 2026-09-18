use crate::error::DirectoryError;
use crate::openers::{data_dirs, file_mime};
use crate::paths::resolve_navigable_path;
use crate::preview::cache_root;
use md5::{Digest, Md5};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::UNIX_EPOCH,
};

const GENERATED_SIZE: &str = "256";
/// Shared-cache sizes in descending preference for a grid cell: 256px is the
/// natural fit, a larger stored picture still scales down cleanly, and the
/// 128px `normal` entry is the last resort.
const SHARED_SIZES: [&str; 4] = ["large", "x-large", "xx-large", "normal"];

struct Thumbnailer {
    exec: String,
    try_exec: Option<String>,
}

fn modified_secs(metadata: &fs::Metadata) -> Option<i64> {
    let modified = metadata.modified().ok()?;
    match modified.duration_since(UNIX_EPOCH) {
        Ok(since) => i64::try_from(since.as_secs()).ok(),
        Err(before) => i64::try_from(before.duration().as_secs()).ok().map(|secs| -secs),
    }
}

/// Every shared-cache entry is named for the MD5 of the file's URI, so the
/// escaping has to match what other file managers wrote.
fn shared_name(target: &Path) -> Option<String> {
    let uri = url::Url::from_file_path(target).ok()?;
    Some(format!("{:x}", Md5::digest(uri.as_str().as_bytes())))
}

/// Reads one `tEXt` keyword out of a PNG; the thumbnail spec attaches
/// `Thumb::URI` and `Thumb::MTime` this way.
fn png_text(bytes: &[u8], keyword: &str) -> Option<String> {
    let mut offset = 8; // Past the PNG signature.

    while offset + 8 <= bytes.len() {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
        let kind = &bytes[offset + 4..offset + 8];
        let start = offset + 8;
        let end = start.checked_add(length)?;
        if end > bytes.len() {
            return None;
        }
        if kind == b"tEXt" {
            let chunk = &bytes[start..end];
            if let Some(split) = chunk.iter().position(|byte| *byte == 0) {
                if &chunk[..split] == keyword.as_bytes() {
                    return String::from_utf8(chunk[split + 1..].to_vec()).ok();
                }
            }
        }
        if kind == b"IEND" {
            return None;
        }
        offset = end + 4; // Past this chunk's CRC.
    }

    None
}

/// A cached thumbnail stays valid only while its recorded `Thumb::MTime` still
/// matches the source, so a rewritten file never keeps showing the old picture.
fn shared_hit(target: &Path, mtime: i64) -> Option<PathBuf> {
    let root = cache_root()?.join("thumbnails");
    let name = format!("{}.png", shared_name(target)?);

    SHARED_SIZES.iter().find_map(|size| {
        let candidate = root.join(size).join(&name);
        let bytes = fs::read(&candidate).ok()?;
        (png_text(&bytes, "Thumb::MTime")? == mtime.to_string()).then_some(candidate)
    })
}

/// Reads the `[Thumbnailer Entry]` group, keeping the entry only when it claims
/// this exact MIME type.
fn parse_thumbnailer(source: &str, mime: &str) -> Option<Thumbnailer> {
    let mut in_entry = false;
    let mut exec = None;
    let mut try_exec = None;
    let mut matched = false;

    for line in source.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            if in_entry {
                break;
            }
            in_entry = line == "[Thumbnailer Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else { continue };
        match key.trim() {
            "Exec" => exec = Some(value.trim().to_owned()),
            "TryExec" => try_exec = Some(value.trim().to_owned()),
            "MimeType" => matched = value.split(';').map(str::trim).any(|candidate| candidate == mime),
            _ => {}
        }
    }

    matched.then(|| Some(Thumbnailer { exec: exec?, try_exec })).flatten()
}

/// `TryExec` names the binary the entry needs: an absolute path has to exist and
/// a bare name has to sit on PATH, or the thumbnailer is not installed.
fn installed(thumbnailer: &Thumbnailer) -> bool {
    let Some(program) = thumbnailer.try_exec.as_deref() else { return true };

    if program.contains('/') {
        return Path::new(program).is_file();
    }
    crate::launch::on_path(program)
}

fn find_thumbnailer(mime: &str) -> Option<Thumbnailer> {
    data_dirs("thumbnailers").into_iter().find_map(|dir| {
        let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "thumbnailer"))
            .collect();
        // Directory order is arbitrary; sort so the chosen entry is stable.
        entries.sort();

        entries.iter().find_map(|path| {
            let found = parse_thumbnailer(&fs::read_to_string(path).ok()?, mime)?;
            installed(&found).then_some(found)
        })
    })
}

/// ponytail: placeholders are standalone tokens in every installed entry;
/// handle embedded ones only if a real `.thumbnailer` ever uses them.
fn build(exec: &str, input: &Path, uri: &str, output: &Path) -> Vec<String> {
    exec.split_whitespace()
        .map(|token| match token {
            "%i" => input.to_string_lossy().into_owned(),
            "%u" => uri.to_owned(),
            "%o" => output.to_string_lossy().into_owned(),
            "%s" => GENERATED_SIZE.to_owned(),
            token => token.trim_matches('"').to_owned(),
        })
        .collect()
}

/// `xdg-mime` spawns a process per call, which a folder of a few hundred files
/// cannot afford, so its answer is reused for every file sharing an extension.
/// Extensionless files still get their own lookup.
fn cached_mime(target: &Path) -> Option<String> {
    static SEEN: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

    let Some(extension) = target.extension().map(|ext| ext.to_string_lossy().to_lowercase()) else {
        return file_mime(target);
    };
    let seen = SEEN.get_or_init(Mutex::default);
    if let Some(known) = seen.lock().ok()?.get(&extension) {
        return known.clone();
    }
    let found = file_mime(target);
    seen.lock().ok()?.insert(extension, found.clone());

    found
}

static STAGE: AtomicU64 = AtomicU64::new(0);

/// Renders through a registered thumbnailer into omafil's own cache, once per
/// (path, mtime, size).
/// ponytail: results are not published back into the shared cache, which would
/// need `Thumb::URI`/`Thumb::MTime` chunks written into the PNG. Add that if
/// other file managers should reuse what omafil rendered.
fn generate(target: &Path, metadata: &fs::Metadata) -> Result<PathBuf, DirectoryError> {
    let dir = cache_root()
        .ok_or_else(|| DirectoryError::detail("No cache directory is available."))?
        .join("omafil/thumbs");
    let mut hasher = DefaultHasher::new();
    target.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    metadata.modified().ok().hash(&mut hasher);
    let key = hasher.finish();
    let png = dir.join(format!("{key:016x}.png"));

    if let Ok(cached) = png.metadata() {
        // An empty file records a thumbnailer that already failed on this exact
        // revision, so scrolling past it again does not re-run the process.
        return match cached.len() {
            0 => Err(DirectoryError::detail("No thumbnail is available for this item.")),
            _ => Ok(png),
        };
    }

    let mime = cached_mime(target).ok_or_else(|| DirectoryError::detail("Unable to determine this file's type."))?;
    let thumbnailer = find_thumbnailer(&mime)
        .ok_or_else(|| DirectoryError::detail("No thumbnailer is installed for this file type."))?;
    let uri = url::Url::from_file_path(target).map_err(|_| DirectoryError::read_failed())?;
    fs::create_dir_all(&dir)?;

    let staged = dir.join(format!("{key:016x}.{}.part.png", STAGE.fetch_add(1, Ordering::Relaxed)));
    let command = build(&thumbnailer.exec, target, uri.as_str(), &staged);
    let (program, arguments) = command
        .split_first()
        .ok_or_else(|| DirectoryError::detail("This thumbnailer has no command."))?;
    let rendered = Command::new(program).args(arguments).output();
    let produced = rendered.is_ok_and(|output| output.status.success())
        && staged.metadata().is_ok_and(|written| written.len() > 0);

    if !produced {
        let _ = fs::remove_file(&staged);
        fs::write(&png, b"")?;
        return Err(DirectoryError::detail("Unable to render a thumbnail for this item."));
    }
    fs::rename(&staged, &png)?;

    Ok(png)
}

/// A PNG thumbnail for one file: the shared freedesktop cache first, so
/// anything another file manager already rendered appears immediately.
pub fn thumbnail(path: String) -> Result<String, DirectoryError> {
    let target = resolve_navigable_path(&path)?;
    let metadata = target.metadata()?;
    if !metadata.is_file() {
        return Err(DirectoryError::detail("Only regular files have thumbnails."));
    }

    let found = modified_secs(&metadata)
        .and_then(|mtime| shared_hit(&target, mtime))
        .map_or_else(|| generate(&target, &metadata), Ok)?;

    Ok(found.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_with(keyword: &str, value: &str) -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut text = keyword.as_bytes().to_vec();
        text.push(0);
        text.extend_from_slice(value.as_bytes());
        bytes.extend_from_slice(&(text.len() as u32).to_be_bytes());
        bytes.extend_from_slice(b"tEXt");
        bytes.extend_from_slice(&text);
        bytes.extend_from_slice(&[0, 0, 0, 0]); // CRC, unchecked while reading.
        bytes.extend_from_slice(&0u32.to_be_bytes());
        bytes.extend_from_slice(b"IEND");
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes
    }

    #[test]
    fn png_text_reads_its_keyword_and_stops_at_the_end_of_the_file() {
        let bytes = png_with("Thumb::MTime", "1750000000");
        assert_eq!(png_text(&bytes, "Thumb::MTime").as_deref(), Some("1750000000"));
        assert_eq!(png_text(&bytes, "Thumb::URI"), None);
        assert_eq!(png_text(&bytes[..12], "Thumb::MTime"), None);
        assert_eq!(png_text(b"not a png at all", "Thumb::MTime"), None);
    }

    #[test]
    fn a_thumbnailer_entry_is_kept_only_for_a_mime_type_it_claims() {
        let source = "[Thumbnailer Entry]\nTryExec=ffmpegthumbnailer\nExec=ffmpegthumbnailer -i %i -o %o -s %s\nMimeType=video/mp4;video/webm;\n";
        let entry = parse_thumbnailer(source, "video/webm").unwrap();
        assert_eq!(entry.try_exec.as_deref(), Some("ffmpegthumbnailer"));
        assert!(parse_thumbnailer(source, "video/webm2").is_none());
        assert!(parse_thumbnailer(source, "image/png").is_none());
        assert!(parse_thumbnailer("[Desktop Entry]\nExec=x\nMimeType=video/mp4;", "video/mp4").is_none());
        assert!(parse_thumbnailer("[Thumbnailer Entry]\nMimeType=video/mp4;", "video/mp4").is_none());
    }

    #[test]
    fn every_placeholder_is_substituted_and_unknown_tokens_survive() {
        let command = build("tool --input %u -i %i -o %o -s %s -c png", Path::new("/tmp/a b.mp4"), "file:///tmp/a%20b.mp4", Path::new("/tmp/out.png"));
        assert_eq!(command, vec!["tool", "--input", "file:///tmp/a%20b.mp4", "-i", "/tmp/a b.mp4", "-o", "/tmp/out.png", "-s", GENERATED_SIZE, "-c", "png"]);
    }

    #[test]
    fn a_missing_try_exec_binary_rejects_the_entry() {
        assert!(installed(&Thumbnailer { exec: "x".into(), try_exec: None }));
        assert!(!installed(&Thumbnailer { exec: "x".into(), try_exec: Some("/nonexistent/binary".into()) }));
        assert!(!installed(&Thumbnailer { exec: "x".into(), try_exec: Some("omafil-no-such-program".into()) }));
    }

    /// Proves the cache key against real entries another file manager wrote:
    /// every stored `Thumb::URI` has to hash back to its own filename, which is
    /// the only check that the URI escaping here matches GIO's.
    #[test]
    #[ignore = "reads the session's populated shared thumbnail cache"]
    fn shared_cache_names_agree_with_the_uris_they_store() {
        let root = cache_root().unwrap().join("thumbnails");
        let mut checked = 0;

        for size in SHARED_SIZES {
            let Ok(entries) = fs::read_dir(root.join(size)) else { continue };
            for entry in entries.flatten().take(50) {
                let path = entry.path();
                if path.extension().is_none_or(|ext| ext != "png") {
                    continue;
                }
                let bytes = fs::read(&path).unwrap();
                let Some(uri) = png_text(&bytes, "Thumb::URI") else { continue };
                let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
                assert_eq!(format!("{:x}", Md5::digest(uri.as_bytes())), stem, "{uri}");

                let Ok(source) = url::Url::parse(&uri).unwrap().to_file_path() else { continue };
                assert_eq!(shared_name(&source).unwrap(), stem, "re-encoding {uri} changed the key");
                checked += 1;
            }
        }
        assert!(checked > 0, "no shared thumbnails to verify");
    }

    /// The whole path against a real installed thumbnailer: a file type with no
    /// preview of its own becomes a PNG.
    #[test]
    #[ignore = "runs the system's installed thumbnailers"]
    fn a_video_with_no_cache_entry_is_rendered_by_an_installed_thumbnailer() {
        let root = tempfile::tempdir().unwrap();
        let video = root.path().join("clip.mp4");
        let made = Command::new("ffmpeg")
            .args(["-y", "-f", "lavfi", "-i", "testsrc=size=320x240:rate=25", "-t", "5", "-pix_fmt", "yuv420p"])
            .arg(&video)
            .output();
        assert!(made.is_ok_and(|output| output.status.success()), "ffmpeg is required for this test");

        let rendered = PathBuf::from(thumbnail(video.to_string_lossy().into_owned()).unwrap());
        assert_eq!(rendered.extension().unwrap(), "png");
        assert_eq!(&fs::read(&rendered).unwrap()[..4], &[0x89, b'P', b'N', b'G']);
        // The second call comes back from the cache without running the process again.
        assert_eq!(thumbnail(video.to_string_lossy().into_owned()).unwrap(), rendered.to_string_lossy());
        // The UI hands this path straight to preview_asset, which has its own allowlist.
        crate::preview::media_asset(&rendered.to_string_lossy()).unwrap();
        fs::remove_file(&rendered).unwrap();
    }
}
