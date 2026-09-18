use crate::{desktop_requests::local_path, error::DirectoryError, paths::resolve_navigable_path};
use std::{
    collections::HashSet,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
use url::Url;

const FILE_CLIPBOARD_TYPE: &str = "x-special/gnome-copied-files";
const URI_LIST_TYPE: &str = "text/uri-list";
const MAX_CLIPBOARD_BYTES: u64 = 64 * 1024 * 1024;
const CLIPBOARD_TIMEOUT: Duration = Duration::from_secs(5);

/// Clipboard owners are external processes: bound both their output and wait time.
fn read_clipboard(args: &[&str]) -> Result<Option<Vec<u8>>, DirectoryError> {
    let mut child = Command::new("wl-paste")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            DirectoryError::detail(
                "Install wl-clipboard to paste files and images from other apps.",
            )
        })?;
    let stdout = child.stdout.take().unwrap();
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout
            .take(MAX_CLIPBOARD_BYTES + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = send.send(result);
    });
    let deadline = Instant::now() + CLIPBOARD_TIMEOUT;
    let bytes = match receive.recv_timeout(CLIPBOARD_TIMEOUT) {
        Ok(Ok(bytes)) if bytes.len() as u64 <= MAX_CLIPBOARD_BYTES => bytes,
        result => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(DirectoryError::detail(if matches!(result, Ok(Ok(_))) {
                "The clipboard image is too large (maximum 64 MiB)."
            } else {
                "The clipboard could not be read. Copy the item again and retry."
            }));
        }
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status.success().then_some(bytes)),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DirectoryError::detail(
                    "The clipboard did not respond. Copy the item again and retry.",
                ));
            }
        }
    }
}

fn read_type(mime: &str) -> Result<Option<Vec<u8>>, DirectoryError> {
    read_clipboard(&["--no-newline", "--type", mime])
}

pub fn read_file_clipboard() -> Result<Option<(Vec<String>, bool)>, DirectoryError> {
    read_file_clipboard_with(read_clipboard)
}

fn read_file_clipboard_with(
    mut read: impl FnMut(&[&str]) -> Result<Option<Vec<u8>>, DirectoryError>,
) -> Result<Option<(Vec<String>, bool)>, DirectoryError> {
    let types = read(&["--list-types"])?.unwrap_or_default();
    let types = String::from_utf8_lossy(&types);
    let has = |mime| types.lines().any(|line| line == mime);
    if has(FILE_CLIPBOARD_TYPE) {
        let bytes = read(&["--no-newline", "--type", FILE_CLIPBOARD_TYPE])?.unwrap_or_default();
        return Ok(parse_file_clipboard(&String::from_utf8_lossy(&bytes)));
    }
    if has(URI_LIST_TYPE) {
        let bytes = read(&["--no-newline", "--type", URI_LIST_TYPE])?.unwrap_or_default();
        let is_cut = has("application/x-kde-cutselection")
            && read(&["--no-newline", "--type", "application/x-kde-cutselection"])?
                .is_some_and(|bytes| bytes == b"1");
        return Ok(parse_uri_list(&String::from_utf8_lossy(&bytes)).map(|paths| (paths, is_cut)));
    }
    Ok(None)
}

fn parse_file_clipboard(payload: &str) -> Option<(Vec<String>, bool)> {
    let (verb, uris) = payload.split_once('\n')?;
    let is_cut = match verb.trim() {
        "cut" => true,
        "copy" => false,
        _ => return None,
    };
    parse_uri_list(uris).map(|paths| (paths, is_cut))
}

fn parse_uri_list(payload: &str) -> Option<Vec<String>> {
    let mut seen = HashSet::new();
    let paths: Vec<String> = payload
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("file:"))
        .filter_map(|line| local_path(line, Path::new("/")).ok())
        .filter(|path| path.symlink_metadata().is_ok())
        .map(|path| path.to_string_lossy().into_owned())
        .filter(|path| seen.insert(path.clone()))
        .collect();
    (!paths.is_empty()).then_some(paths)
}

pub fn paste_clipboard_image(destination: &str) -> Result<Option<String>, DirectoryError> {
    let types = read_clipboard(&["--list-types"])?.unwrap_or_default();
    let types = String::from_utf8_lossy(&types);
    for (mime, extension) in [
        ("image/png", "png"),
        ("image/jpeg", "jpg"),
        ("image/webp", "webp"),
    ] {
        if !types.lines().any(|line| line == mime) {
            continue;
        }
        let bytes = read_type(mime)?.ok_or_else(|| {
            DirectoryError::detail("The copied image is no longer available. Copy it again.")
        })?;
        return save_image(destination, extension, &bytes).map(Some);
    }
    Ok(None)
}

fn save_image(destination: &str, extension: &str, bytes: &[u8]) -> Result<String, DirectoryError> {
    let valid = match extension {
        "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" => bytes.starts_with(b"\xff\xd8\xff"),
        "webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
        _ => false,
    };
    if !valid || bytes.len() as u64 > MAX_CLIPBOARD_BYTES {
        return Err(DirectoryError::detail(
            "The clipboard does not contain a supported image.",
        ));
    }
    let directory = resolve_navigable_path(destination)?;
    let mut scratch = tempfile::NamedTempFile::new_in(&directory).map_err(DirectoryError::from)?;
    scratch.write_all(bytes).map_err(DirectoryError::from)?;
    scratch.as_file().sync_all().map_err(DirectoryError::from)?;
    for number in 1..=10000 {
        let name = if number == 1 {
            format!("Clipboard image.{extension}")
        } else {
            format!("Clipboard image ({number}).{extension}")
        };
        let path = directory.join(name);
        match scratch.persist_noclobber(&path) {
            Ok(_) => return Ok(path.to_string_lossy().into_owned()),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                scratch = error.file
            }
            Err(error) => return Err(DirectoryError::from(error.error)),
        }
    }
    Err(DirectoryError::detail(
        "Unable to find a free name for the copied image.",
    ))
}

pub fn write_file_clipboard(paths: &[String], is_cut: bool) -> Result<(), DirectoryError> {
    let mut payload = String::from(if is_cut { "cut\n" } else { "copy\n" });
    let uris: Vec<String> = paths
        .iter()
        .filter_map(|path| Url::from_file_path(path).ok())
        .map(|url| url.to_string())
        .collect();
    payload.push_str(&uris.join("\n"));
    let mut child = Command::new("wl-copy")
        .args(["--type", FILE_CLIPBOARD_TYPE])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            DirectoryError::detail("Install wl-clipboard so other apps can see copied files.")
        })?;
    let result = child
        .stdin
        .take()
        .ok_or_else(|| DirectoryError::detail("The clipboard could not be written."))?
        .write_all(payload.as_bytes());
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(DirectoryError::detail(
            "The clipboard could not be written.",
        ));
    }
    // wl-copy forks its serving process once it owns the clipboard. Reap the
    // launcher so a following Paste cannot race clipboard publication.
    let deadline = Instant::now() + CLIPBOARD_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DirectoryError::detail(
                    "The desktop clipboard could not be updated.",
                ));
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_paths_and_the_verb_another_file_manager_wrote() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("note.txt");
        std::fs::write(&file, "hi").unwrap();
        let uri = Url::from_file_path(&file).unwrap();

        let (paths, is_cut) = parse_file_clipboard(&format!("cut\n{uri}")).unwrap();

        assert_eq!(paths, vec![file.to_string_lossy().into_owned()]);
        assert!(is_cut);
        assert!(parse_file_clipboard(&format!("copy\n{uri}")).unwrap().1 == false);
    }

    #[test]
    fn ignores_a_clipboard_holding_something_other_than_files() {
        assert!(parse_file_clipboard("").is_none());
        assert!(parse_file_clipboard("copy\n").is_none());
        assert!(parse_file_clipboard("copy\nfile:///does/not/exist").is_none());
    }
    #[test]
    fn uri_lists_handle_comments_unicode_duplicates_and_dangling_links() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("hello # ሰላም%.txt");
        std::fs::write(&file, "hello").unwrap();
        let link = directory.path().join("broken-link");
        std::os::unix::fs::symlink("missing", &link).unwrap();
        let uri = Url::from_file_path(&file).unwrap();
        let link_uri = Url::from_file_path(&link).unwrap();
        let payload = format!("# comment\r\n{uri}\r\n{uri}\r\n{link_uri}\r\nsftp://host/file\r\nfile://remote/etc/passwd\r\nfile:///etc/passwd?bad\r\n");
        assert_eq!(
            parse_uri_list(&payload).unwrap(),
            vec![file.to_string_lossy(), link.to_string_lossy()]
        );
        assert!(parse_file_clipboard(&format!("unknown\n{uri}")).is_none());
    }

    #[test]
    fn pasted_images_preserve_bytes_and_never_replace_files_or_symlinks() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().to_str().unwrap();
        let original = directory.path().join("original.png");
        std::fs::write(&original, b"untouched").unwrap();
        std::os::unix::fs::symlink(&original, directory.path().join("Clipboard image.png"))
            .unwrap();
        let bytes = b"\x89PNG\r\n\x1a\nimage payload";
        let first = save_image(destination, "png", bytes).unwrap();
        let second = save_image(destination, "png", bytes).unwrap();
        assert!(first.ends_with("Clipboard image (2).png"));
        assert!(second.ends_with("Clipboard image (3).png"));
        assert_eq!(std::fs::read(first).unwrap(), bytes);
        assert_eq!(std::fs::read(original).unwrap(), b"untouched");
    }

    #[test]
    fn rejects_invalid_images_and_non_directory_destinations() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().to_str().unwrap();
        assert!(save_image(destination, "png", b"not an image").is_err());
        assert!(save_image(destination, "../png", b"\x89PNG\r\n\x1a\n").is_err());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
        let file = directory.path().join("file");
        std::fs::write(&file, b"unchanged").unwrap();
        assert!(save_image(file.to_str().unwrap(), "jpg", b"\xff\xd8\xfftest").is_err());
        assert_eq!(std::fs::read(file).unwrap(), b"unchanged");
    }
    #[test]
    fn reads_uri_list_and_kde_cut_offers_without_a_gnome_payload() {
        let directory = tempfile::tempdir().unwrap();
        let uri = Url::from_directory_path(directory.path())
            .unwrap()
            .to_string();
        for cut in [false, true] {
            let value = read_file_clipboard_with(|args| {
                Ok(Some(match args.last().copied().unwrap() {
                    "--list-types" => b"text/uri-list\napplication/x-kde-cutselection\n".to_vec(),
                    URI_LIST_TYPE => uri.as_bytes().to_vec(),
                    "application/x-kde-cutselection" => {
                        if cut {
                            b"1".to_vec()
                        } else {
                            b"0".to_vec()
                        }
                    }
                    _ => panic!("unexpected clipboard request"),
                }))
            })
            .unwrap()
            .unwrap();
            assert_eq!(value.0, vec![format!("{}/", directory.path().display())]);
            assert_eq!(value.1, cut);
        }
        assert!(
            read_file_clipboard_with(|_| Ok(Some(b"image/png\n".to_vec())))
                .unwrap()
                .is_none()
        );
        assert!(
            read_file_clipboard_with(|_| Err(DirectoryError::detail("clipboard unavailable")))
                .is_err()
        );
    }
}
