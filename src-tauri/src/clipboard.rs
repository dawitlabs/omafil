use crate::error::DirectoryError;
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
use url::Url;

/// The type every desktop file manager agrees on for copied files. Its payload is
/// `copy` or `cut` on the first line, then one `file://` URI per line.
const FILE_CLIPBOARD_TYPE: &str = "x-special/gnome-copied-files";

// ponytail: wl-clipboard only, which is what Omarchy runs. Add an xclip branch if X11 sessions matter.
pub(crate) fn read_file_clipboard() -> Option<(Vec<String>, bool)> {
    let output = Command::new("wl-paste")
        .args(["--no-newline", "--type", FILE_CLIPBOARD_TYPE])
        .output()
        .ok()
        .filter(|output| output.status.success())?;

    parse_file_clipboard(&String::from_utf8_lossy(&output.stdout))
}

fn parse_file_clipboard(payload: &str) -> Option<(Vec<String>, bool)> {
    let mut lines = payload.lines();
    let is_cut = lines.next()?.trim() == "cut";
    let paths: Vec<String> = lines
        .filter_map(|line| Url::parse(line.trim()).ok())
        .filter_map(|url| url.to_file_path().ok())
        .filter(|path: &PathBuf| path.exists())
        .map(|path| path.to_string_lossy().into_owned())
        .collect();

    (!paths.is_empty()).then_some((paths, is_cut))
}

pub(crate) fn write_file_clipboard(paths: &[String], is_cut: bool) -> Result<(), DirectoryError> {
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
        .map_err(|_| DirectoryError::detail("wl-clipboard is not installed, so other apps cannot see the copy."))?;

    child
        .stdin
        .take()
        .ok_or_else(|| DirectoryError::detail("The clipboard could not be written."))?
        .write_all(payload.as_bytes())
        .map_err(|_| DirectoryError::detail("The clipboard could not be written."))
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
}
