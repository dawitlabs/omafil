use crate::error::DirectoryError;
use crate::paths::resolve_navigable_path;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{mpsc, Mutex},
    thread,
    time::Duration,
};

/// A burst of writes lands as many events; this is how long the walk waits for
/// quiet before reporting a single change.
const SETTLE: Duration = Duration::from_millis(200);

/// Opening or reading a file produces access notifications. Those do not
/// change the folder listing and must never cause a UI refresh.
fn changes_directory_contents(event: &Event) -> bool {
    matches!(event.kind, EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_))
}

#[derive(Default)]
pub(crate) struct DirectoryWatcher {
    active: Mutex<HashMap<String, RecommendedWatcher>>,
}

impl DirectoryWatcher {
    /// Makes the watched set equal to `paths`: dropped watchers end their
    /// threads, folders already watched are left alone.
    pub(crate) fn sync<F>(&self, paths: &[String], on_change: F) -> Result<(), DirectoryError>
    where
        F: Fn(String) + Send + Clone + 'static,
    {
        let mut active = self.active.lock().map_err(|_| DirectoryError::read_failed())?;
        active.retain(|path, _| paths.contains(path));

        for path in paths {
            if active.contains_key(path) {
                continue;
            }
            let directory = resolve_navigable_path(path)?;
            if !directory.is_dir() {
                continue;
            }
            let watcher = start_watch(directory, on_change.clone()).map_err(|_| DirectoryError::read_failed())?;
            active.insert(path.clone(), watcher);
        }

        Ok(())
    }

    pub(crate) fn stop(&self) {
        if let Ok(mut active) = self.active.lock() {
            active.clear();
        }
    }
}

pub(crate) fn start_watch<F>(directory: PathBuf, on_change: F) -> notify::Result<RecommendedWatcher>
where
    F: Fn(String) + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        if let Ok(event) = event {
            if changes_directory_contents(&event) {
                let _ = sender.send(());
            }
        }
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
    use super::{changes_directory_contents, start_watch};
    use notify::{event::{AccessKind, AccessMode, CreateKind}, Event, EventKind};
    use std::{fs, sync::mpsc, time::Duration};

    #[test]
    fn opening_a_file_does_not_refresh_its_folder() {
        let opened = Event::new(EventKind::Access(AccessKind::Open(AccessMode::Any)));
        let created = Event::new(EventKind::Create(CreateKind::Any));

        assert!(!changes_directory_contents(&opened));
        assert!(changes_directory_contents(&created));
    }

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
