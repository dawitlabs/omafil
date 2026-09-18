//! Opt-in, reversible per-user MIME and D-Bus registration.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const DESKTOP_ID: &str = "omafil.desktop";
const OWN_DEFAULT: &str = "omafil.desktop;";
const SERVICE: &str = "org.freedesktop.FileManager1.service";

#[derive(Clone)]
pub(crate) struct IntegrationPaths {
    config: PathBuf,
    data: PathBuf,
    desktop: String,
}

#[derive(Serialize, Deserialize)]
struct Registration {
    version: u32,
    mime_file: String,
    previous_default: Option<String>,
    previous_service: Option<Vec<u8>>,
    installed_service: Vec<u8>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationStatus {
    pub is_default: bool,
    pub can_enable: bool,
    pub can_restore: bool,
    pub activation_installed: bool,
    pub service_active: bool,
    pub reason: Option<String>,
}

impl IntegrationPaths {
    pub fn current() -> Result<Self, String> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or("Home directory is unavailable.")?;
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"));
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .split(':')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        Ok(Self {
            config,
            data,
            desktop,
        })
    }
    fn manifest(&self) -> PathBuf {
        self.config.join("omafil/desktop-integration.json")
    }
    fn service(&self) -> PathBuf {
        self.data.join("dbus-1/services").join(SERVICE)
    }
    fn mime_filename(&self) -> String {
        if !self.desktop.is_empty()
            && self
                .desktop
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            format!("{}-mimeapps.list", self.desktop)
        } else {
            "mimeapps.list".into()
        }
    }
    fn registered(&self) -> Result<Option<Registration>, String> {
        let Some(bytes) = read_optional(&self.manifest())? else {
            return Ok(None);
        };
        let registration: Registration = serde_json::from_slice(&bytes).map_err(|_| {
            "The saved desktop registration is unreadable. No settings were changed."
        })?;
        if registration.version != 1
            || Path::new(&registration.mime_file)
                .file_name()
                .and_then(|n| n.to_str())
                != Some(registration.mime_file.as_str())
            || !registration.mime_file.ends_with("mimeapps.list")
        {
            return Err(
                "The saved desktop registration is invalid. No settings were changed.".into(),
            );
        }
        Ok(Some(registration))
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() => {
            return Err(format!(
                "{} is not a regular file. Configure this managed or linked file manually.",
                path.display()
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
        _ => {}
    }
    fs::read(path).map(Some).map_err(|e| e.to_string())
}

fn read_text(path: &Path) -> Result<String, String> {
    String::from_utf8(read_optional(path)?.unwrap_or_default())
        .map_err(|_| format!("{} is not UTF-8 text.", path.display()))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing configuration directory.")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    // Never replace a symlink or other special file belonging to a dotfile manager.
    read_optional(path)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(bytes)
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn restore_file(path: &Path, bytes: Option<&[u8]>) -> Result<(), String> {
    if let Some(bytes) = bytes {
        return write_atomic(path, bytes);
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn default_entry(source: &str) -> Option<String> {
    let mut in_defaults = false;
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_defaults = line == "[Default Applications]";
        }
        if in_defaults {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "inode/directory" {
                    return Some(value.trim().to_owned());
                }
            }
        }
    }
    None
}

/// Edits exactly one MIME key, preserving other groups, associations and comments.
fn set_default_entry(source: &str, value: Option<&str>) -> String {
    let mut lines = Vec::new();
    let mut in_defaults = false;
    let mut written = false;
    let mut found_group = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_defaults && !written {
                if let Some(value) = value {
                    lines.push(format!("inode/directory={value}"));
                }
                written = true;
            }
            in_defaults = trimmed == "[Default Applications]";
            found_group |= in_defaults;
        }
        if in_defaults
            && trimmed
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == "inode/directory")
        {
            if !written {
                if let Some(value) = value {
                    lines.push(format!("inode/directory={value}"));
                }
                written = true;
            }
            continue;
        }
        lines.push(line.to_owned());
    }
    if !written {
        if let Some(value) = value {
            if !found_group {
                lines.push("[Default Applications]".into());
            }
            lines.push(format!("inode/directory={value}"));
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn service_contents(executable: &Path) -> Result<Vec<u8>, String> {
    let executable = executable
        .to_str()
        .ok_or("The executable path is not UTF-8.")?;
    if executable.contains(['\n', '\r', '\0']) {
        return Err("The executable path contains unsupported characters.".into());
    }
    // D-Bus Exec uses shell-style argument parsing, without invoking a shell.
    let quoted = executable.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!(
        "[D-BUS Service]\nName=org.freedesktop.FileManager1\nExec=\"{quoted}\" --desktop-service\n"
    )
    .into_bytes())
}

fn installed_executable(paths: &IntegrationPaths) -> Result<PathBuf, String> {
    let executable = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let installed = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join("omafil"))
        .find(|path| path.is_file())
        .and_then(|path| path.canonicalize().ok());
    if installed.as_ref() != Some(&executable) {
        return Err("Install this version of Omafil and launch it from your app menu before making it the default.".into());
    }
    let data_dirs =
        std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    let desktop_entry = std::iter::once(paths.data.clone())
        .chain(std::env::split_paths(&data_dirs))
        .map(|dir| dir.join("applications").join(DESKTOP_ID))
        .find(|entry| entry.is_file())
        .ok_or("Install Omafil's desktop entry before making it the default.")?;
    let entry = fs::read_to_string(desktop_entry).map_err(|e| e.to_string())?;
    if !entry.lines().any(|line| {
        matches!(
            line.trim(),
            "Exec=omafil %U" | "Exec=omafil %u" | "Exec=omafil -- %U"
        )
    }) || entry.lines().any(|line| line.trim() == "Hidden=true")
    {
        return Err("The Omafil desktop entry is customized or hidden. Reinstall its desktop entry before changing the default.".into());
    }
    Ok(executable)
}

fn lock(paths: &IntegrationPaths) -> Result<fs::File, String> {
    let directory = paths.config.join("omafil");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(directory.join("desktop-integration.lock"))
        .map_err(|e| e.to_string())?;
    rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive)
        .map_err(|e| e.to_string())?;
    Ok(file)
}

pub(crate) fn enable() -> Result<(), String> {
    let paths = IntegrationPaths::current()?;
    let executable = installed_executable(&paths)?;
    enable_at(&paths, &executable)
}

fn enable_at(paths: &IntegrationPaths, executable: &Path) -> Result<(), String> {
    let _lock = lock(paths)?;
    if let Some(saved) = paths.registered()? {
        if read_optional(&paths.service())?.as_deref() == Some(&saved.installed_service)
            && default_entry(&read_text(&paths.config.join(&saved.mime_file))?).as_deref()
                == Some(OWN_DEFAULT)
        {
            return Ok(());
        }
        return Err("Desktop settings changed after registration. Restore the previous setup before enabling again.".into());
    }
    let mime_file = paths.mime_filename();
    let mime_path = paths.config.join(&mime_file);
    let original_mime = read_text(&mime_path)?;
    let saved = Registration {
        version: 1,
        mime_file,
        previous_default: default_entry(&original_mime),
        previous_service: read_optional(&paths.service())?,
        installed_service: service_contents(executable)?,
    };
    write_atomic(
        &paths.manifest(),
        &serde_json::to_vec_pretty(&saved).map_err(|e| e.to_string())?,
    )?;
    let applied = write_atomic(
        &mime_path,
        set_default_entry(&original_mime, Some(OWN_DEFAULT)).as_bytes(),
    )
    .and_then(|_| write_atomic(&paths.service(), &saved.installed_service));
    if let Err(error) = applied {
        // The manifest is retained if rollback fails so Restore can recover it.
        if restore_at_locked(paths, &saved).is_err() {
            return Err(format!(
                "Could not finish setup: {error}. Use Restore previous setup to recover."
            ));
        }
        return Err(format!(
            "Could not finish setup; previous settings were restored: {error}"
        ));
    }
    Ok(())
}

pub(crate) fn restore() -> Result<(), String> {
    let paths = IntegrationPaths::current()?;
    let _lock = lock(&paths)?;
    let saved = paths
        .registered()?
        .ok_or("There is no saved desktop setup to restore.")?;
    restore_at_locked(&paths, &saved)
}

fn restore_at_locked(paths: &IntegrationPaths, saved: &Registration) -> Result<(), String> {
    let current_service = read_optional(&paths.service())?;
    // Leave a service that another application/user has subsequently replaced.
    if current_service.as_deref() == Some(&saved.installed_service) {
        restore_file(&paths.service(), saved.previous_service.as_deref())?;
    }
    let mime_path = paths.config.join(&saved.mime_file);
    let current_mime = read_text(&mime_path)?;
    if default_entry(&current_mime).as_deref() == Some(OWN_DEFAULT) {
        write_atomic(
            &mime_path,
            set_default_entry(&current_mime, saved.previous_default.as_deref()).as_bytes(),
        )?;
    }
    restore_file(&paths.manifest(), None)
}

pub(crate) fn status() -> Result<IntegrationStatus, String> {
    let paths = IntegrationPaths::current()?;
    let installed = installed_executable(&paths);
    let saved = paths.registered()?;
    let activation_installed = match &saved {
        Some(saved) => {
            read_optional(&paths.service())?.as_deref() == Some(&saved.installed_service)
        }
        None => false,
    };
    Ok(IntegrationStatus {
        is_default: crate::file_manager_service::is_default(),
        can_enable: installed.is_ok(),
        can_restore: saved.is_some(),
        activation_installed,
        service_active: false,
        reason: installed.err(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn paths(root: &Path) -> IntegrationPaths {
        IntegrationPaths {
            config: root.join("config"),
            data: root.join("data"),
            desktop: "hyprland".into(),
        }
    }

    #[test]
    fn edits_only_the_folder_default_and_restores_absence() {
        let source = "# preferences\n[Default Applications]\ntext/plain=editor.desktop;\ninode/directory=old.desktop;\n[Added Associations]\ninode/directory=extra.desktop;\n";
        let edited = set_default_entry(source, Some(OWN_DEFAULT));
        assert_eq!(default_entry(&edited).as_deref(), Some(OWN_DEFAULT));
        assert!(edited.contains("text/plain=editor.desktop;"));
        assert!(edited.contains("[Added Associations]\ninode/directory=extra.desktop;"));
        assert_eq!(set_default_entry(&edited, Some("old.desktop;")), source);
        assert_eq!(default_entry(&set_default_entry(&edited, None)), None);
    }

    #[test]
    fn setup_is_idempotent_and_restore_preserves_later_unrelated_edits() {
        let root = tempfile::tempdir().unwrap();
        let paths = paths(root.path());
        let mime = paths.config.join(paths.mime_filename());
        write_atomic(
            &mime,
            b"[Default Applications]\ninode/directory=old.desktop;\n",
        )
        .unwrap();
        write_atomic(&paths.service(), b"previous service\n").unwrap();
        enable_at(&paths, Path::new("/opt/My Files/omafil")).unwrap();
        enable_at(&paths, Path::new("/opt/My Files/omafil")).unwrap();
        let mut current = read_text(&mime).unwrap();
        current.push_str("text/plain=new-editor.desktop;\n");
        write_atomic(&mime, current.as_bytes()).unwrap();
        let saved = paths.registered().unwrap().unwrap();
        restore_at_locked(&paths, &saved).unwrap();
        assert_eq!(
            default_entry(&read_text(&mime).unwrap()).as_deref(),
            Some("old.desktop;")
        );
        assert!(read_text(&mime)
            .unwrap()
            .contains("text/plain=new-editor.desktop;"));
        assert_eq!(
            read_optional(&paths.service()).unwrap().unwrap(),
            b"previous service\n"
        );
        assert!(paths.registered().unwrap().is_none());
    }

    #[test]
    fn restore_never_overwrites_a_later_default_or_service_choice() {
        let root = tempfile::tempdir().unwrap();
        let paths = paths(root.path());
        enable_at(&paths, Path::new("/usr/bin/omafil")).unwrap();
        let saved = paths.registered().unwrap().unwrap();
        write_atomic(
            &paths.config.join(&saved.mime_file),
            b"[Default Applications]\ninode/directory=other.desktop;\n",
        )
        .unwrap();
        write_atomic(&paths.service(), b"another provider\n").unwrap();
        restore_at_locked(&paths, &saved).unwrap();
        assert_eq!(
            read_optional(&paths.service()).unwrap().unwrap(),
            b"another provider\n"
        );
        assert_eq!(
            default_entry(&read_text(&paths.config.join(&saved.mime_file)).unwrap()).as_deref(),
            Some("other.desktop;")
        );
    }

    #[test]
    fn linked_files_are_rejected_without_modifying_the_target() {
        let root = tempfile::tempdir().unwrap();
        let paths = paths(root.path());
        fs::create_dir_all(&paths.config).unwrap();
        let target = root.path().join("dotfile");
        fs::write(&target, "untouched").unwrap();
        std::os::unix::fs::symlink(&target, paths.config.join(paths.mime_filename())).unwrap();
        assert!(enable_at(&paths, Path::new("/usr/bin/omafil")).is_err());
        assert_eq!(fs::read_to_string(target).unwrap(), "untouched");
        assert!(!paths.manifest().exists());
    }

    #[test]
    fn activation_quotes_spaces_and_rejects_newlines() {
        let service =
            String::from_utf8(service_contents(Path::new("/opt/My Files/omafil")).unwrap())
                .unwrap();
        assert!(service.contains("Exec=\"/opt/My Files/omafil\" --desktop-service"));
        assert!(service_contents(Path::new("/tmp/bad\nname")).is_err());
    }

    #[test]
    fn failed_service_publication_rolls_back_the_mime_change() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let paths = paths(root.path());
        let mime = paths.config.join(paths.mime_filename());
        write_atomic(
            &mime,
            b"[Default Applications]\ninode/directory=old.desktop;\n",
        )
        .unwrap();
        let service_parent = paths.service().parent().unwrap().to_path_buf();
        fs::create_dir_all(&service_parent).unwrap();
        fs::set_permissions(&service_parent, fs::Permissions::from_mode(0o500)).unwrap();
        let result = enable_at(&paths, Path::new("/usr/bin/omafil"));
        fs::set_permissions(&service_parent, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(result.is_err());
        assert_eq!(
            default_entry(&read_text(&mime).unwrap()).as_deref(),
            Some("old.desktop;")
        );
        assert!(!paths.manifest().exists());
        assert!(!paths.service().exists());
    }
}
