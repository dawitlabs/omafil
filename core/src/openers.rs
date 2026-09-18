use crate::error::DirectoryError;
use crate::launch::spawn_first;
use crate::paths::resolve_navigable_path;
use serde::Serialize;
use std::{collections::HashSet, fs, path::{Path, PathBuf}, process::Command};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opener {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

struct DesktopEntry {
    name: String,
    exec: String,
    mime_types: Vec<String>,
}

/// Every XDG data directory's `sub` folder, most specific first.
pub fn data_dirs(sub: &str) -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".local/share")));
    let data_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());

    data_home
        .into_iter()
        .chain(data_dirs.split(':').filter(|dir| !dir.is_empty()).map(PathBuf::from))
        .map(|dir| dir.join(sub))
        .collect()
}

/// Reads the `[Desktop Entry]` group only; hidden and non-application entries are dropped.
fn parse_desktop_entry(source: &str) -> Option<DesktopEntry> {
    let mut in_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut mime_types = Vec::new();

    for line in source.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            if in_entry {
                break;
            }
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else { continue };
        match key.trim() {
            "Name" => name = Some(value.trim().to_owned()),
            "Exec" => exec = Some(value.trim().to_owned()),
            "MimeType" => mime_types = value.split(';').map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned).collect(),
            "NoDisplay" | "Hidden" if value.trim() == "true" => return None,
            "Type" if value.trim() != "Application" => return None,
            _ => {}
        }
    }

    Some(DesktopEntry { name: name?, exec: exec?, mime_types })
}

/// Expands the Exec line for one file; the file is appended when the entry
/// names no placeholder at all.
fn build_command(exec: &str, path: &str) -> Vec<String> {
    let mut has_placeholder = false;
    let mut command: Vec<String> = exec
        .split_whitespace()
        .filter_map(|token| match token {
            "%f" | "%F" | "%u" | "%U" => {
                has_placeholder = true;
                Some(path.to_owned())
            }
            "%%" => Some("%".to_owned()),
            token if token.starts_with('%') && token.len() == 2 => None,
            token => Some(token.trim_matches('"').to_owned()),
        })
        .collect();

    if !has_placeholder {
        command.push(path.to_owned());
    }

    command
}

fn xdg_mime(args: &[&str]) -> Option<String> {
    let output = Command::new("xdg-mime").args(args).output().ok()?;
    let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();

    (output.status.success() && !value.is_empty()).then_some(value)
}

pub fn file_mime(target: &Path) -> Option<String> {
    xdg_mime(&["query", "filetype", &target.to_string_lossy()])
}

fn find_entry(id: &str) -> Option<DesktopEntry> {
    data_dirs("applications")
        .into_iter()
        .find_map(|dir| fs::read_to_string(dir.join(id)).ok())
        .and_then(|source| parse_desktop_entry(&source))
}

pub fn list_openers(path: String) -> Result<Vec<Opener>, DirectoryError> {
    let target = resolve_navigable_path(&path)?;
    let mime = file_mime(&target)
        .ok_or_else(|| DirectoryError::detail("Unable to determine this file's type."))?;
    let default_id = xdg_mime(&["query", "default", &mime]);
    let mut seen = HashSet::new();
    let mut openers = Vec::new();

    for dir in data_dirs("applications") {
        let Ok(entries) = fs::read_dir(dir) else { continue };

        for entry in entries.flatten() {
            let id = entry.file_name().to_string_lossy().into_owned();
            if !id.ends_with(".desktop") || seen.contains(&id) {
                continue;
            }
            let Some(parsed) = fs::read_to_string(entry.path()).ok().and_then(|s| parse_desktop_entry(&s)) else { continue };
            if !parsed.mime_types.iter().any(|candidate| candidate == &mime) {
                continue;
            }

            seen.insert(id.clone());
            openers.push(Opener { is_default: default_id.as_deref() == Some(&id), name: parsed.name, id });
        }
    }

    openers.sort_by(|a, b| b.is_default.cmp(&a.is_default).then_with(|| a.name.cmp(&b.name)));

    Ok(openers)
}

pub fn set_default_opener(path: String, desktop_id: String) -> Result<(), DirectoryError> {
    let target = resolve_navigable_path(&path)?;
    find_entry(&desktop_id).ok_or_else(|| DirectoryError::detail("That app is no longer installed."))?;
    let mime = file_mime(&target)
        .ok_or_else(|| DirectoryError::detail("Unable to determine this file's type."))?;
    let applied = Command::new("xdg-mime").args(["default", &desktop_id, &mime]).status();

    match applied {
        Ok(status) if status.success() => Ok(()),
        _ => Err(DirectoryError::detail("Unable to change the default app.")),
    }
}

pub fn open_with(path: String, desktop_id: String) -> Result<(), DirectoryError> {
    let target = resolve_navigable_path(&path)?;
    let entry = find_entry(&desktop_id).ok_or_else(|| DirectoryError::detail("That app is no longer installed."))?;
    let command = build_command(&entry.exec, &target.to_string_lossy());
    let scoped = ["uwsm-app".to_owned(), "--".to_owned()].into_iter().chain(command.iter().cloned()).collect();
    let directory = target.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));

    spawn_first(&[scoped, command], &directory, "Unable to start that app.")
}

#[cfg(test)]
mod tests {
    use super::{build_command, parse_desktop_entry};

    #[test]
    fn parses_entries_and_expands_exec_placeholders() {
        let entry = parse_desktop_entry("[Desktop Entry]\nType=Application\nName=Image Viewer\nExec=viewer --quiet %U\nMimeType=image/png;image/jpeg;\n[Desktop Action Foo]\nName=Ignored\n").expect("entry");

        assert_eq!(entry.name, "Image Viewer");
        assert_eq!(entry.mime_types, vec!["image/png", "image/jpeg"]);
        assert!(parse_desktop_entry("[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true\n").is_none());
        assert_eq!(build_command(&entry.exec, "/tmp/a.png"), vec!["viewer", "--quiet", "/tmp/a.png"]);
        assert_eq!(build_command("app %i %c", "/tmp/a"), vec!["app", "/tmp/a"]);
    }
}
