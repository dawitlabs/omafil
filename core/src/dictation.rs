//! Voice dictation through voxtype, the tool Omarchy already ships. The daemon
//! types into whatever holds keyboard focus, so this only starts it and reports
//! what it is doing.
use crate::error::DirectoryError;
use crate::launch::{on_path, spawn_first};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

const PROGRAM: &str = "voxtype";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationStatus {
    pub is_available: bool,
    /// "idle", "recording" or "transcribing", as the daemon last wrote it.
    pub state: String,
}

fn state_path() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(|dir| PathBuf::from(dir).join("voxtype/state"))
}

/// The daemon rewrites this file on every transition, so polling it costs a read
/// rather than a `voxtype status` process. Anything unrecognised counts as idle:
/// a stale or missing file must never leave the button stuck listening.
fn read_state(path: Option<&Path>) -> String {
    path.and_then(|path| fs::read_to_string(path).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| matches!(value.as_str(), "recording" | "transcribing"))
        .unwrap_or_else(|| "idle".to_owned())
}

pub fn dictation_status() -> DictationStatus {
    DictationStatus {
        is_available: on_path(PROGRAM),
        state: read_state(state_path().as_deref()),
    }
}

pub fn toggle_dictation() -> Result<(), DirectoryError> {
    spawn_first(
        &[vec![PROGRAM.into(), "record".into(), "toggle".into()]],
        Path::new("/"),
        "Install voxtype to dictate into the search box.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_daemons_active_states_are_reported() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");

        assert_eq!(read_state(None), "idle");
        assert_eq!(read_state(Some(&state)), "idle", "a missing file is not listening");
        for (written, expected) in [
            ("recording\n", "recording"),
            ("transcribing", "transcribing"),
            ("idle", "idle"),
            ("", "idle"),
            ("something-else", "idle"),
        ] {
            fs::write(&state, written).unwrap();
            assert_eq!(read_state(Some(&state)), expected, "state file held {written:?}");
        }
    }
}
