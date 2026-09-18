//! Validated local desktop opening requests. No target is executed as a command.
use crate::paths::{resolve_entry_path, resolve_navigable_path};
use serde::Serialize;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub(crate) const MAX_TARGETS: usize = 128;
const MAX_PENDING: usize = 256;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenRequest {
    pub targets: Vec<OpenTarget>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenTarget {
    pub folder: String,
    pub selection: Vec<String>,
    pub properties: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum OpenMode {
    Open,
    Folders,
    Select,
    Properties,
}

pub(crate) enum CliAction {
    Help,
    Version,
    DesktopService,
    MakeDefault,
    RestoreDefault,
    Open(OpenRequest),
}

pub(crate) fn local_path(value: &str, cwd: &Path) -> Result<PathBuf, String> {
    if value.is_empty() || value.contains('\0') {
        return Err("A location cannot be empty or contain a NUL character.".into());
    }
    let path = if value.starts_with("file:") {
        // Url accepts malformed percent escapes literally; reject those in URIs,
        // while retaining literal percent characters in ordinary Linux paths.
        let bytes = value.as_bytes();
        for (i, byte) in bytes.iter().enumerate() {
            if *byte == b'%'
                && !(bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                    && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit))
            {
                return Err("The file URI contains an invalid percent escape.".into());
            }
        }
        let uri = url::Url::parse(value).map_err(|_| "Invalid file URI.")?;
        if uri.query().is_some()
            || uri.fragment().is_some()
            || !uri.username().is_empty()
            || uri.password().is_some()
            || uri.port().is_some()
        {
            return Err(
                "File URIs cannot contain credentials, a port, a query or a fragment.".into(),
            );
        }
        uri.to_file_path().map_err(|_| {
            "Only local file URIs are supported; remote locations are not available yet."
        })?
    } else {
        if value.contains("://") {
            return Err(
                "Remote locations are not available yet. Open a local path or file:// URI.".into(),
            );
        }
        PathBuf::from(value)
    };
    Ok(if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    })
}

pub(crate) fn request_for(
    values: &[String],
    cwd: &Path,
    mode: OpenMode,
) -> Result<OpenRequest, String> {
    if values.len() > MAX_TARGETS {
        return Err(format!("Open at most {MAX_TARGETS} items at once."));
    }
    if mode == OpenMode::Properties && values.len() != 1 {
        return Err("Open properties for one item at a time.".into());
    }
    let mut targets: Vec<OpenTarget> = Vec::new();
    for value in values {
        let path = local_path(value, cwd)?;
        let path_text = path
            .to_str()
            .ok_or("This filename cannot be displayed as UTF-8.")?;
        let reveal = mode == OpenMode::Select
            || mode == OpenMode::Properties
            || (mode == OpenMode::Open && !path.is_dir());
        let resolved = if reveal && path.parent().is_some() {
            resolve_entry_path(path_text)
        } else {
            resolve_navigable_path(path_text)
        }
        .map_err(|error| format!("Cannot open {}: {}", path.display(), error.message()))?;
        let folder = if reveal {
            resolved.parent().unwrap_or(&resolved).to_path_buf()
        } else {
            if !resolved.is_dir() {
                return Err("Show folders requires a directory.".into());
            }
            resolved.clone()
        };
        let folder = folder
            .to_str()
            .ok_or("This folder cannot be displayed as UTF-8.")?
            .to_owned();
        let item = resolved
            .to_str()
            .ok_or("This filename cannot be displayed as UTF-8.")?
            .to_owned();
        let selection = if reveal && item != folder {
            vec![item.clone()]
        } else {
            vec![]
        };
        if let Some(existing) = targets.iter_mut().find(|target| target.folder == folder) {
            for path in selection {
                if !existing.selection.contains(&path) {
                    existing.selection.push(path);
                }
            }
        } else {
            targets.push(OpenTarget {
                folder,
                selection,
                properties: (mode == OpenMode::Properties).then_some(item),
            });
        }
    }
    Ok(OpenRequest {
        targets,
        error: None,
    })
}

pub(crate) fn parse_cli(args: &[String], cwd: &Path) -> Result<CliAction, String> {
    if args.len() == 1 {
        match args[0].as_str() {
            "--desktop-service" => return Ok(CliAction::DesktopService),
            "--make-default" => return Ok(CliAction::MakeDefault),
            "--restore-default" => return Ok(CliAction::RestoreDefault),
            _ => {}
        }
    }
    let mut mode = OpenMode::Open;
    let mut literal = false;
    let mut values = Vec::new();
    for arg in args {
        if !literal {
            match arg.as_str() {
                "--" => {
                    literal = true;
                    continue;
                }
                "--help" | "-h" => return Ok(CliAction::Help),
                "--version" | "-V" => return Ok(CliAction::Version),
                "--select" => {
                    mode = OpenMode::Select;
                    continue;
                }
                "--properties" => {
                    mode = OpenMode::Properties;
                    continue;
                }
                value if value.starts_with('-') => {
                    return Err(format!(
                        "Unknown option: {value}. Use -- before a filename beginning with '-'."
                    ))
                }
                _ => {}
            }
        }
        values.push(arg.clone());
    }
    if mode != OpenMode::Open && values.is_empty() {
        return Err("This option requires an item to open.".into());
    }
    request_for(&values, cwd, mode).map(CliAction::Open)
}

#[derive(Default)]
pub(crate) struct OpenRequests(Mutex<VecDeque<OpenRequest>>);

impl OpenRequests {
    pub fn push(&self, request: OpenRequest) -> Result<(), String> {
        let mut pending = self
            .0
            .lock()
            .map_err(|_| "Unable to queue the opening request.")?;
        let count: usize = pending.iter().map(|entry| entry.targets.len().max(1)).sum();
        if count + request.targets.len().max(1) > MAX_PENDING {
            return Err(
                "Too many pending requests. Wait for Omafil to finish opening folders.".into(),
            );
        }
        pending.push_back(request);
        Ok(())
    }

    pub fn drain(&self) -> Vec<OpenRequest> {
        self.0
            .lock()
            .map(|mut pending| pending.drain(..).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn decodes_file_uris_but_preserves_literal_path_characters() {
        let cwd = Path::new("/tmp");
        assert_eq!(
            local_path("file:///tmp/a%20b%23c%25%C3%A9", cwd).unwrap(),
            Path::new("/tmp/a b#c%é")
        );
        assert_eq!(
            local_path("file://localhost/tmp/a", cwd).unwrap(),
            Path::new("/tmp/a")
        );
        assert_eq!(
            local_path("a%20b#c", cwd).unwrap(),
            Path::new("/tmp/a%20b#c")
        );
        for invalid in [
            "file://server/tmp/a",
            "smb://server/share",
            "file:///tmp/a?x=1",
            "file:///tmp/a#part",
            "file:///tmp/a%xy",
            "file:///tmp/a%",
        ] {
            assert!(local_path(invalid, cwd).is_err(), "{invalid}");
        }
    }

    #[test]
    fn groups_selection_and_preserves_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::os::unix::fs::symlink("missing", dir.path().join("link")).unwrap();
        let request = request_for(
            &["a.txt".into(), "link".into(), "a.txt".into()],
            dir.path(),
            OpenMode::Select,
        )
        .unwrap();
        assert_eq!(request.targets.len(), 1);
        assert_eq!(request.targets[0].selection.len(), 2);
        assert!(request.targets[0].selection[1].ends_with("/link"));
        assert!(request_for(&["a.txt".into()], dir.path(), OpenMode::Folders).is_err());
        assert!(request_for(&["missing".into()], dir.path(), OpenMode::Open).is_err());
    }

    #[test]
    fn handles_cli_flags_and_literal_dash_names() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("-folder")).unwrap();
        assert!(parse_cli(&["--select".into()], dir.path()).is_err());
        assert!(parse_cli(&["--bogus".into()], dir.path()).is_err());
        assert!(matches!(
            parse_cli(&["--".into(), "-folder".into()], dir.path()),
            Ok(CliAction::Open(_))
        ));
        assert!(matches!(
            parse_cli(&["--help".into()], dir.path()),
            Ok(CliAction::Help)
        ));
        assert!(request_for(
            &vec!["/".into(); MAX_TARGETS + 1],
            dir.path(),
            OpenMode::Open
        )
        .is_err());
        assert!(request_for(&["/".into()], dir.path(), OpenMode::Select).is_ok());
    }

    #[test]
    fn queued_startup_requests_are_delivered_once_and_bounded() {
        let queue = OpenRequests::default();
        for _ in 0..MAX_PENDING {
            queue.push(OpenRequest::default()).unwrap();
        }
        assert!(queue.push(OpenRequest::default()).is_err());
        assert_eq!(queue.drain().len(), MAX_PENDING);
        assert!(queue.drain().is_empty());
        assert!(queue.push(OpenRequest::default()).is_ok());
    }
}
