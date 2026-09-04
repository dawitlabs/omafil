use crate::error::DirectoryError;
use crate::paths::resolve_navigable_path;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::PathBuf,
    sync::{mpsc, Mutex},
    thread,
    time::Duration,
};

/// A burst of writes lands as many events; this is how long the walk waits for
/// quiet before reporting a single change.
const SETTLE: Duration = Duration::from_millis(200);

#[derive(Default)]
pub(crate) struct DirectoryWatcher {
    active: Mutex<Option<RecommendedWatcher>>,
}

impl DirectoryWatcher {
    pub(crate) fn watch<F>(&self, path: &str, on_change: F) -> Result<(), DirectoryError>
    where
        F: Fn(String) + Send + 'static,
    {
        let directory = resolve_navigable_path(path)?;

        if !directory.is_dir() {
            return Err(DirectoryError::unavailable());
        }

        let watcher = start_watch(directory, on_change).map_err(|_| DirectoryError::read_failed())?;
        // Dropping the previous watcher ends its thread, so only one directory
        // is ever being reported on.
        *self.active.lock().map_err(|_| DirectoryError::read_failed())? = Some(watcher);

        Ok(())
    }

    pub(crate) fn stop(&self) {
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
    }
}

pub(crate) fn start_watch<F>(directory: PathBuf, on_change: F) -> notify::Result<RecommendedWatcher>
where
    F: Fn(String) + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })?;

    watcher.watch(&directory, RecursiveMode::NonRecursive)?;

    let reported = directory.to_string_lossy().into_owned();

    thread::spawn(move || {
        // recv fails once the watcher is dropped, which ends this thread.
        while receiver.recv().is_ok() {
            while receiver.recv_timeout(SETTLE).is_ok() {}

            on_change(reported.clone());
        }
    });

    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::start_watch;
    use std::{fs, sync::mpsc, time::Duration};

    #[test]
    fn a_new_file_is_reported_once_the_directory_settles() {
        let directory = std::env::temp_dir().join(format!("omafil-watch-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();

        let (sender, receiver) = mpsc::channel();
        let watcher = start_watch(directory.clone(), move |path| {
            let _ = sender.send(path);
        })
        .unwrap();

        fs::write(directory.join("created.txt"), b"hello").unwrap();

        let reported = receiver.recv_timeout(Duration::from_secs(5));
        drop(watcher);
        fs::remove_dir_all(&directory).ok();

        assert_eq!(reported.unwrap(), directory.to_string_lossy());
    }
}
