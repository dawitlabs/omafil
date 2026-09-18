use crate::error::RecentFilesError;
use crate::paths::current_user_home_directory;
use quick_xml::{events::Event, Reader, XmlVersion};
use serde::Serialize;
use std::{fs, path::PathBuf};
use url::Url;

pub const MAX_RECENT_FILES: usize = 50;

const EMPTY_HISTORY: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<xbel version=\"1.0\"\n",
    "      xmlns:bookmark=\"http://www.freedesktop.org/standards/desktop-bookmarks\"\n",
    "      xmlns:mime=\"http://www.freedesktop.org/standards/shared-mime-info\">\n",
    "</xbel>\n"
);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFile {
    name: String,
    path: String,
    parent_directory: String,
}

pub fn recently_used_file_path() -> Result<PathBuf, RecentFilesError> {
    current_user_home_directory()
        .map(|home_directory| home_directory.join(".local/share/recently-used.xbel"))
        .map_err(|_| RecentFilesError::unavailable())
}

pub fn read_recent_files() -> Result<Vec<RecentFile>, RecentFilesError> {
    let history_path = recently_used_file_path()?;
    if !history_path.exists() {
        return Ok(Vec::new());
    }

    let history = fs::read_to_string(history_path).map_err(|_| RecentFilesError::unavailable())?;
    parse_history(&history)
}

pub fn clear_recent_files() -> Result<(), RecentFilesError> {
    let history_path = recently_used_file_path()?;
    if !history_path.exists() {
        return Ok(());
    }

    fs::write(history_path, EMPTY_HISTORY).map_err(|_| RecentFilesError::clear_failed())
}

fn parse_history(history: &str) -> Result<Vec<RecentFile>, RecentFilesError> {
    let mut reader = Reader::from_str(history);
    let mut recent_files = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) if event.name().as_ref() == b"bookmark" => {
                let mut href = None;
                let mut modified = None;

                for attribute in event.attributes().flatten() {
                    match attribute.key.as_ref() {
                        b"href" => {
                            href = attribute
                                .normalized_value(XmlVersion::Explicit1_0)
                                .ok()
                                .map(|value| value.into_owned())
                        }
                        b"modified" => {
                            modified = attribute
                                .normalized_value(XmlVersion::Explicit1_0)
                                .ok()
                                .map(|value| value.into_owned())
                        }
                        _ => {}
                    }
                }

                let Some((href, modified)) = href.zip(modified) else {
                    continue;
                };
                let Ok(url) = Url::parse(&href) else {
                    continue;
                };
                let Ok(path) = url.to_file_path() else {
                    continue;
                };
                if !path.is_file() {
                    continue;
                }

                let Some(name) = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                else {
                    continue;
                };
                let parent_directory = path
                    .parent()
                    .map(|parent| parent.to_string_lossy().into_owned())
                    .unwrap_or_default();

                recent_files.push((
                    modified,
                    RecentFile {
                        name,
                        path: path.to_string_lossy().into_owned(),
                        parent_directory,
                    },
                ));
            }
            Ok(Event::Eof) => break,
            Err(_) => return Err(RecentFilesError::unavailable()),
            _ => {}
        }
    }

    recent_files.sort_by(|left, right| right.0.cmp(&left.0));
    recent_files.truncate(MAX_RECENT_FILES);
    Ok(recent_files.into_iter().map(|(_, file)| file).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleared_history_lists_no_files() {
        assert!(parse_history(EMPTY_HISTORY).unwrap().is_empty());
    }

    #[test]
    fn populated_history_lists_existing_files() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("note.txt");
        fs::write(&file, "hi").unwrap();
        let href = Url::from_file_path(&file).unwrap();
        let history = format!(
            "<?xml version=\"1.0\"?><xbel><bookmark href=\"{href}\" modified=\"2026-01-01T00:00:00Z\"><info/></bookmark></xbel>"
        );

        let files = parse_history(&history).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "note.txt");
    }
}
