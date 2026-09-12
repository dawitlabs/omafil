use crate::archive::{create_zip_with_context, extract_zip_with_context};
use crate::error::DirectoryError;
use crate::operation_io::{OperationContext, OperationProgress};
use crate::operations::{transfer_with_context, TransferConflictPolicy, TransferResult};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
};
use tauri::{AppHandle, Emitter};

const EVENT_NAME: &str = "file-operation";
type Task = Box<
    dyn FnOnce(&mut OperationContext<'_>, &mut Vec<TransferResult>) -> Result<(), DirectoryError>
        + Send,
>;
type Cancellations = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

/// A single FIFO worker avoids competing jobs modifying the same destination.
pub(crate) struct OperationQueue {
    next_id: AtomicU64,
    cancellations: Cancellations,
    sender: mpsc::Sender<Job>,
}

struct Job {
    app: AppHandle,
    update: OperationUpdate,
    cancellation: Arc<AtomicBool>,
    task: Task,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueuedOperation {
    id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum OperationState {
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationUpdate {
    id: String,
    kind: &'static str,
    state: OperationState,
    total_items: usize,
    #[serde(flatten)]
    progress: OperationProgress,
    results: Vec<TransferResult>,
    error: Option<String>,
}

fn execute(
    mut update: OperationUpdate,
    cancellation: &AtomicBool,
    task: Task,
    mut notify: impl FnMut(&OperationUpdate),
) -> OperationUpdate {
    let mut running = update.clone();
    running.state = OperationState::Running;
    notify(&running);
    let mut context = OperationContext::new(cancellation, |progress| {
        running.progress = progress.clone();
        notify(&running);
    });
    let result = context
        .check()
        .and_then(|()| task(&mut context, &mut update.results));
    update.progress = context.progress.clone();
    update.state = match result {
        Ok(()) => OperationState::Completed,
        Err(error) if error.is_cancelled() => OperationState::Cancelled,
        Err(error) => {
            update.error = Some(error.message().to_owned());
            OperationState::Failed
        }
    };
    update
}

impl Default for OperationQueue {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel::<Job>();
        let cancellations: Cancellations = Arc::new(Mutex::new(HashMap::new()));
        let worker_cancellations = Arc::clone(&cancellations);
        std::thread::spawn(move || {
            for job in receiver {
                let final_update = execute(job.update, &job.cancellation, job.task, |update| {
                    let _ = job.app.emit(EVENT_NAME, update);
                });
                worker_cancellations
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .remove(&final_update.id);
                let _ = job.app.emit(EVENT_NAME, final_update);
            }
        });
        Self {
            next_id: AtomicU64::new(0),
            cancellations,
            sender,
        }
    }
}

impl OperationQueue {
    fn start(
        &self,
        app: AppHandle,
        kind: &'static str,
        total_items: usize,
        task: Task,
    ) -> QueuedOperation {
        let id = format!(
            "operation-{}",
            self.next_id.fetch_add(1, Ordering::Relaxed) + 1
        );
        let cancellation = Arc::new(AtomicBool::new(false));
        self.cancellations
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .insert(id.clone(), Arc::clone(&cancellation));
        let update = OperationUpdate {
            id: id.clone(),
            kind,
            state: OperationState::Queued,
            total_items,
            progress: OperationProgress::default(),
            results: Vec::new(),
            error: None,
        };
        let _ = app.emit(EVENT_NAME, &update);
        if let Err(error) = self.sender.send(Job {
            app,
            update,
            cancellation,
            task,
        }) {
            let mut job = error.0;
            self.cancellations
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .remove(&id);
            job.update.state = OperationState::Failed;
            job.update.error = Some(
                "The file operation worker is unavailable. Restart Omafil and try again.".into(),
            );
            let _ = job.app.emit(EVENT_NAME, job.update);
        }
        QueuedOperation { id }
    }

    pub(crate) fn start_archive_create(
        &self,
        app: AppHandle,
        paths: Vec<String>,
        destination_path: String,
        name: String,
    ) -> QueuedOperation {
        self.start(
            app,
            "compress",
            1,
            Box::new(move |context, _| {
                create_zip_with_context(paths, destination_path, name, context).map(|_| ())
            }),
        )
    }

    pub(crate) fn start_archive_extract(
        &self,
        app: AppHandle,
        path: String,
        destination_path: String,
    ) -> QueuedOperation {
        self.start(
            app,
            "extract",
            1,
            Box::new(move |context, _| extract_zip_with_context(path, destination_path, context)),
        )
    }

    pub(crate) fn start_transfer(
        &self,
        app: AppHandle,
        paths: Vec<String>,
        destination_path: String,
        is_move: bool,
        conflict_policy: TransferConflictPolicy,
    ) -> QueuedOperation {
        self.start(
            app,
            if is_move { "move" } else { "copy" },
            paths.len(),
            Box::new(move |context, results| {
                transfer_with_context(
                    paths,
                    destination_path,
                    is_move,
                    conflict_policy,
                    context,
                    results,
                )
            }),
        )
    }

    pub(crate) fn cancel(&self, id: &str) -> bool {
        let registry = self
            .cancellations
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let Some(cancellation) = registry.get(id) else {
            return false;
        };
        cancellation.store(true, Ordering::Relaxed);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queued() -> OperationUpdate {
        OperationUpdate {
            id: "test".into(),
            kind: "move",
            state: OperationState::Queued,
            total_items: 2,
            progress: OperationProgress::default(),
            results: Vec::new(),
            error: None,
        }
    }

    #[test]
    fn partial_results_survive_failure_and_cancellation() {
        for cancelled in [false, true] {
            let cancel = AtomicBool::new(false);
            let update = execute(
                queued(),
                &cancel,
                Box::new(move |context, results| {
                    results.push(TransferResult {
                        source_path: "source".into(),
                        destination_path: "target".into(),
                        skipped: false,
                    });
                    context.complete_item();
                    Err(if cancelled {
                        DirectoryError::cancelled()
                    } else {
                        DirectoryError::detail("Disk full")
                    })
                }),
                |_| {},
            );
            assert_eq!(update.results.len(), 1);
            assert_eq!(update.progress.completed_items, 1);
            assert_eq!(
                update.state,
                if cancelled {
                    OperationState::Cancelled
                } else {
                    OperationState::Failed
                }
            );
            if !cancelled {
                assert_eq!(update.error.as_deref(), Some("Disk full"));
            }
        }
    }

    #[test]
    fn cancelled_queued_job_never_executes() {
        let update = execute(
            queued(),
            &AtomicBool::new(true),
            Box::new(|_, _| panic!("must not execute")),
            |_| {},
        );
        assert_eq!(update.state, OperationState::Cancelled);
    }
}
