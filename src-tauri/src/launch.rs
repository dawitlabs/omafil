use crate::error::DirectoryError;
use crate::paths::resolve_navigable_path;
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};

/// True when a bare program name resolves on PATH.
pub(crate) fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| dir.join(program).is_file())
    })
}

/// Runs the first candidate that exists on PATH. A child that fails after
/// starting reports through its own UI, so only a missing program moves on.
pub(crate) fn spawn_first(candidates: &[Vec<String>], cwd: &Path, failure: &str) -> Result<(), DirectoryError> {
    for argv in candidates {
        let Some((program, args)) = argv.split_first() else { continue };
        let spawned = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match spawned {
            Ok(mut child) => {
                // Reaping in the background keeps launcher shims from lingering as zombies.
                thread::spawn(move || {
                    let _ = child.wait();
                });
                return Ok(());
            }
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(_) => return Err(DirectoryError::detail(failure)),
        }
    }

    Err(DirectoryError::detail(failure))
}

fn containing_directory(target: PathBuf) -> Result<PathBuf, DirectoryError> {
    if target.is_dir() {
        return Ok(target);
    }

    target.parent().map(Path::to_path_buf).ok_or_else(DirectoryError::unavailable)
}

pub(crate) fn open_terminal(path: &str) -> Result<(), DirectoryError> {
    let directory = containing_directory(resolve_navigable_path(path)?)?;
    let dir_flag = format!("--dir={}", directory.display());
    let mut candidates = vec![
        vec!["uwsm-app".into(), "--".into(), "xdg-terminal-exec".into(), dir_flag.clone()],
        vec!["xdg-terminal-exec".into(), dir_flag],
    ];

    if let Some(terminal) = std::env::var_os("TERMINAL") {
        candidates.push(vec![terminal.to_string_lossy().into_owned()]);
    }

    spawn_first(&candidates, &directory, "No terminal is available to open here.")
}

pub(crate) fn open_editor(path: &str) -> Result<(), DirectoryError> {
    let target = resolve_navigable_path(path)?;
    let file = target.to_string_lossy().into_owned();
    let directory = containing_directory(target)?;

    spawn_first(
        &[vec!["omarchy-launch-editor".into(), file]],
        &directory,
        "No editor is available. This needs Omarchy's launch editor.",
    )
}

#[cfg(test)]
mod tests {
    use super::spawn_first;
    use std::path::Path;

    #[test]
    fn skips_missing_programs_and_reports_when_none_exist() {
        let missing = vec!["omafil-definitely-missing-program".to_owned()];
        let present = vec!["true".to_owned()];

        assert!(spawn_first(&[missing.clone(), present], Path::new("/"), "none").is_ok());
        assert!(spawn_first(&[missing], Path::new("/"), "none").is_err());
    }
}
