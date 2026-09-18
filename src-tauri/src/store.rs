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

#[derive(Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Settings {
    #[serde(deserialize_with = "deserialize_server_uris")]
    pub(crate) server_uris: Vec<String>,
    pub(crate) font_scale: u16,
    pub(crate) show_hidden: bool,
    pub(crate) theme: String,
    pub(crate) default_sort: String,
    pub(crate) default_descending: bool,
    pub(crate) vim_keys: bool,
}

fn safe_server_uri(uri: &str) -> bool {
    if uri.len() > 8192 || uri.chars().any(char::is_control) {
        return false;
    }
    let Ok(parsed) = url::Url::parse(uri) else {
        return false;
    };
    matches!(
        parsed.scheme(),
        "sftp" | "smb" | "dav" | "davs" | "ftp" | "ftps"
    ) && parsed.host_str().is_some()
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none()
}

fn deserialize_server_uris<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<String>, D::Error> {
    let values = Vec::<String>::deserialize(deserializer)?;
    let mut servers = Vec::new();
    for uri in values.into_iter().filter(|uri| safe_server_uri(uri)) {
        if !servers.contains(&uri) {
            servers.push(uri);
        }
        if servers.len() == 16 {
            break;
        }
    }
    Ok(servers)
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_uris: Vec::new(),
            font_scale: 100,
            show_hidden: false,
            theme: "system".to_owned(),
            default_sort: "name".to_owned(),
            default_descending: false,
            vim_keys: false,
        }
    }
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
        .or_else(|| {
            current_user_home_directory()
                .ok()
                .map(|home| home.join(".config"))
        })
        .map(|config| config.join("omafil"))
}

pub(crate) fn read_state() -> AppState {
    state_directory()
        .map(|directory| directory.join("state.json"))
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

pub(crate) fn write_state(mut state: AppState) -> Result<(), StoreError> {
    state
        .settings
        .server_uris
        .retain(|uri| safe_server_uri(uri));
    state.settings.server_uris.truncate(16);
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
        assert_eq!(state.settings.font_scale, 100);
        assert_eq!(state.settings.theme, "system");
        assert_eq!(state.settings.default_sort, "name");
    }

    #[test]
    fn font_size_survives_state_serialization() {
        let mut state = AppState::default();
        state.settings.font_scale = 125;
        let json = serde_json::to_string(&state).unwrap();
        let loaded: AppState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.settings.font_scale, 125);
        assert!(json.contains("\"fontScale\":125"));
    }

    #[test]
    fn a_corrupt_file_falls_back_to_defaults() {
        assert!(serde_json::from_str::<AppState>("{ not json").is_err());
    }
    #[test]
    fn saved_servers_exclude_credentials_and_unsupported_locations() {
        let state: AppState = serde_json::from_str(r#"{"settings":{"serverUris":["sftp://server/folder","sftp://user:secret@host/","smb://user@host/share","https://host/","sftp://host/?token=secret","sftp://server/folder"]}}"#).unwrap();
        assert_eq!(state.settings.server_uris, vec!["sftp://server/folder"]);
        let stored = serde_json::to_string(&state).unwrap();
        assert!(!stored.contains("secret"));
        let loaded: AppState = serde_json::from_str(&stored).unwrap();
        assert_eq!(loaded.settings.server_uris, state.settings.server_uris);
    }
}
