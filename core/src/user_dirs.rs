//! Reads the XDG user-directory file as data; never sources it in a shell.
use crate::error::DirectoryError;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn resolve(location: &str, home: &Path) -> Result<PathBuf, DirectoryError> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home.join(".config"))
        .join("user-dirs.dirs");
    let contents = match fs::read_to_string(config) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    resolve_from_config(location, home, contents.as_deref())
}

fn resolve_from_config(
    location: &str,
    home: &Path,
    config: Option<&str>,
) -> Result<PathBuf, DirectoryError> {
    let (key, fallback) = match location {
        "desktop" => ("XDG_DESKTOP_DIR", "Desktop"),
        "documents" => ("XDG_DOCUMENTS_DIR", "Documents"),
        "downloads" => ("XDG_DOWNLOAD_DIR", "Downloads"),
        "pictures" => ("XDG_PICTURES_DIR", "Pictures"),
        "videos" => ("XDG_VIDEOS_DIR", "Videos"),
        "music" => ("XDG_MUSIC_DIR", "Music"),
        _ => return Err(DirectoryError::unavailable()),
    };
    let configured = config
        .into_iter()
        .flat_map(str::lines)
        .filter_map(|line| {
            let (name, value) = line.trim().split_once('=')?;
            (name.trim() == key).then_some(value.trim())
        })
        .last();
    let path = match configured {
        Some(value) => parse_path(value, home).ok_or_else(|| {
            DirectoryError::detail("This folder has an invalid XDG user-directory setting.")
        })?,
        None => home.join(fallback),
    };
    let path = path.canonicalize()?;
    if path == home.canonicalize()? {
        return Err(DirectoryError::detail(
            "This standard folder is disabled in your desktop settings.",
        ));
    }
    if !path.metadata()?.is_dir() {
        return Err(DirectoryError::detail(
            "The configured location is not a folder.",
        ));
    }
    Ok(path)
}

fn parse_path(value: &str, home: &Path) -> Option<PathBuf> {
    let value = value.strip_prefix('"')?;
    let (prefix, value) = if let Some(rest) = value.strip_prefix("$HOME") {
        if !rest.starts_with('/') && !rest.starts_with('"') {
            return None;
        }
        (Some(home), rest)
    } else {
        (None, value)
    };
    let mut chars = value.chars();
    let mut suffix = String::new();
    let mut closed = false;
    while let Some(character) = chars.next() {
        match character {
            '"' => {
                closed = true;
                break;
            }
            '\\' => {
                let escaped = chars.next()?;
                if !matches!(escaped, '\\' | '"' | '$' | '`') {
                    suffix.push('\\');
                }
                suffix.push(escaped);
            }
            '$' | '`' | '\0' => return None,
            other => suffix.push(other),
        }
    }
    let trailing = chars.as_str().trim();
    if !closed || (!trailing.is_empty() && !trailing.starts_with('#')) {
        return None;
    }
    let path = match prefix {
        Some(home) => home.join(suffix.trim_start_matches('/')),
        None => PathBuf::from(suffix),
    };
    path.is_absolute().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_localized_and_external_paths_without_execution() {
        let home = Path::new("/home/test");
        assert_eq!(
            parse_path(r#""$HOME/Mes documents""#, home),
            Some(home.join("Mes documents"))
        );
        assert_eq!(
            parse_path(r#""/srv/Shared Documents" # comment"#, home),
            Some(PathBuf::from("/srv/Shared Documents"))
        );
        assert_eq!(
            parse_path(r#""$HOME/a\"b\$c\\d""#, home),
            Some(home.join("a\"b$c\\d"))
        );
        for invalid in [
            r#""relative/path""#,
            r#""$OTHER/path""#,
            r#""$(touch /tmp/no)""#,
            r#""`command`""#,
            r#""$HOMEoops/path""#,
            r#""/valid"; command"#,
            "\"/unclosed",
        ] {
            assert!(parse_path(invalid, home).is_none(), "{invalid}");
        }
    }

    #[test]
    fn resolves_existing_locations_and_keeps_disabled_or_missing_unavailable() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        fs::create_dir_all(home.join("Documents")).unwrap();
        let outside = root.path().join("Shared Documents");
        fs::create_dir(&outside).unwrap();
        let config = format!("XDG_DOCUMENTS_DIR=\"{}\"", outside.display());
        assert_eq!(
            resolve_from_config("documents", &home, Some(&config)).unwrap(),
            outside
        );
        assert_eq!(
            resolve_from_config("documents", &home, None).unwrap(),
            home.join("Documents")
        );
        assert!(
            resolve_from_config("documents", &home, Some("XDG_DOCUMENTS_DIR=\"$HOME/\"")).is_err()
        );
        assert!(resolve_from_config(
            "documents",
            &home,
            Some("XDG_DOCUMENTS_DIR=\"$HOME/Missing\"")
        )
        .is_err());
        assert!(!home.join("Missing").exists());
        assert!(
            resolve_from_config("documents", &home, Some("XDG_DOCUMENTS_DIR=\"relative\""))
                .is_err()
        );
    }
}
