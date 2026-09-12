use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LOG_BYTES: u64 = 1024 * 1024;

#[derive(Serialize)]
struct Entry<'a> {
    at: u64,
    event: &'a str,
    message: &'a str,
    detail: Option<&'a str>,
}

pub(crate) fn log_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))?;

    Some(base.join("omafil/errors.log"))
}

/// One JSON object per line; the previous log is kept once when the cap is hit.
pub(crate) fn append(path: &PathBuf, event: &str, message: &str, detail: Option<&str>) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if path.metadata().map(|m| m.len() > MAX_LOG_BYTES).unwrap_or(false) {
        let _ = fs::rename(path, path.with_extension("log.1"));
    }
    let at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let Ok(line) = serde_json::to_string(&Entry { at, event, message, detail }) else { return };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

pub(crate) fn install_panic_hook() {
    let previous = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        if let Some(path) = log_path() {
            let location = info.location().map(|l| format!("{}:{}", l.file(), l.line()));
            append(&path, "panic", &info.to_string(), location.as_deref());
        }
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::append;

    #[test]
    fn appends_json_lines_and_creates_parents() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested/errors.log");

        append(&path, "client_error", "boom", Some("stack"));
        append(&path, "panic", "again", None);

        let lines: Vec<String> = std::fs::read_to_string(&path).expect("log").lines().map(String::from).collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"event\":\"client_error\"") && lines[0].contains("\"detail\":\"stack\""));
        assert!(lines[1].contains("\"detail\":null"));
    }
}
