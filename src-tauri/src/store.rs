use crate::paths::current_user_home_directory;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf};

#[derive(Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct AppState {
    pub(crate) pins: Vec<PinnedLocation>,
    pub(crate) tags: Vec<Tag>,
    pub(crate) tagged: BTreeMap<String, Vec<String>>,
    pub(crate) settings: Settings,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PinnedLocation {
    pub(crate) path: String,
    pub(crate) label: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Tag {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) color: String,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Settings {
    pub(crate) show_hidden: bool,
}

#[derive(Serialize)]
pub(crate) struct StoreError {
    code: &'static str,
    message: &'static str,
}

impl StoreError {
    pub(crate) const fn write_failed() -> Self {
        Self {
            code: "state_write_failed",
            message: "Unable to save your pins, tags and settings.",
        }
    }
}

fn state_directory() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| current_user_home_directory().ok().map(|home| home.join(".config")))
        .map(|config| config.join("omafil"))
}

pub(crate) fn read_state() -> AppState {
    state_directory()
        .map(|directory| directory.join("state.json"))
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

pub(crate) fn write_state(state: AppState) -> Result<(), StoreError> {
    let directory = state_directory().ok_or_else(StoreError::write_failed)?;
    fs::create_dir_all(&directory).map_err(|_| StoreError::write_failed())?;

    let contents = serde_json::to_string_pretty(&state).map_err(|_| StoreError::write_failed())?;
    let scratch = directory.join("state.json.tmp");

    fs::write(&scratch, contents).map_err(|_| StoreError::write_failed())?;
    fs::rename(scratch, directory.join("state.json")).map_err(|_| StoreError::write_failed())
}

#[cfg(test)]
mod tests {
    use super::AppState;

    #[test]
    fn unknown_and_missing_fields_do_not_break_loading() {
        let stored = r#"{"pins":[{"path":"/home/dave/Code","label":"Code"}],"unknownField":42}"#;
        let state: AppState = serde_json::from_str(stored).unwrap();

        assert_eq!(state.pins.len(), 1);
        assert_eq!(state.pins[0].label, "Code");
        assert!(state.tags.is_empty());
        assert!(!state.settings.show_hidden);
    }

    #[test]
    fn a_corrupt_file_falls_back_to_defaults() {
        assert!(serde_json::from_str::<AppState>("{ not json").is_err());
    }
}
