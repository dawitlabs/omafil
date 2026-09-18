//! Isolated optional Linux providers. Remote addresses never enter local path APIs.
use crate::error::DirectoryError;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, State};

const BRIDGE: &str = include_str!("../services/linux_services.py");
const MAX_OUTPUT: u64 = 8 * 1024 * 1024;

#[derive(Default)]
pub(crate) struct ServiceRequests(Mutex<HashMap<String, Arc<AtomicBool>>>);

impl ServiceRequests {
    fn begin(&self, id: &str) -> Result<Arc<AtomicBool>, DirectoryError> {
        if id.is_empty()
            || id.len() > 80
            || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err(DirectoryError::detail(
                "Invalid service request identifier.",
            ));
        }
        let mut requests = self
            .0
            .lock()
            .map_err(|_| DirectoryError::operation_failed())?;
        if requests.len() >= 16 || requests.contains_key(id) {
            return Err(DirectoryError::detail(
                "Too many service requests. Cancel an existing request and retry.",
            ));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        requests.insert(id.into(), cancel.clone());
        Ok(cancel)
    }

    fn finish(&self, id: &str) {
        if let Ok(mut requests) = self.0.lock() {
            requests.remove(id);
        }
    }
}

fn run(
    operation: &str,
    payload: Value,
    cancelled: Arc<AtomicBool>,
    progress: impl Fn(Value) + Send + 'static,
) -> Result<Value, DirectoryError> {
    let timeout = match operation {
        "connect" => Duration::from_secs(180),
        "copy" => Duration::from_secs(1800),
        "capabilities" | "mounts" | "list" | "indexed-search" => Duration::from_secs(30),
        _ => return Err(DirectoryError::detail("Unsupported Linux service request.")),
    };
    let bytes = serde_json::to_vec(&payload).map_err(|_| DirectoryError::operation_failed())?;
    if bytes.len() > 1024 * 1024 {
        return Err(DirectoryError::detail("This service request is too large."));
    }
    let mut child = Command::new("/usr/bin/python3")
        .args(["-I", "-c", BRIDGE, operation])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            DirectoryError::detail(
                "Install python and python-gobject to enable Linux network and search services.",
            )
        })?;
    let mut stdin = child.stdin.take().unwrap();
    // Writing is on its own thread so even a stalled helper remains cancellable.
    std::thread::spawn(move || {
        let _ = stdin.write_all(&bytes);
    });
    let stdout = child.stdout.take().unwrap();
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout.take(MAX_OUTPUT + 1));
        let mut result = None;
        let mut count = 0;
        for line in reader.lines() {
            let Ok(line) = line else { break };
            count += line.len() as u64 + 1;
            if count > MAX_OUTPUT {
                break;
            }
            let Ok(value) = serde_json::from_str::<Value>(&line) else {
                break;
            };
            if let Some(update) = value.get("progress") {
                progress(update.clone());
            } else {
                result = Some(value);
            }
        }
        let _ = send.send(if count > MAX_OUTPUT { None } else { result });
    });
    let deadline = Instant::now() + timeout;
    loop {
        if cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(DirectoryError::detail(
                if cancelled.load(Ordering::Relaxed) {
                    "Operation cancelled. Completed files were kept; check the destination for incomplete files."
                } else {
                    "The Linux service timed out. Check the connection and retry. A transfer may have left incomplete files at its destination."
                },
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = receive.recv_timeout(Duration::from_secs(1)).ok().flatten();
                if status.success() {
                    if let Some(value) = output {
                        if let Some(error) = value.get("error").and_then(Value::as_str) {
                            return Err(DirectoryError::detail(error));
                        }
                        if let Some(result) = value.get("result") {
                            return Ok(result.clone());
                        }
                    }
                }
                return Err(DirectoryError::detail("The Linux provider is unavailable. Install python-gobject and the required GVfs or LocalSearch packages."));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(DirectoryError::operation_failed());
            }
        }
    }
}

fn describe_indexed_results(result: Value) -> Result<Value, DirectoryError> {
    let mut entries = Vec::new();
    let mut skipped = 0;
    for uri in result
        .get("uris")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let path = uri
            .as_str()
            .and_then(|uri| url::Url::parse(uri).ok())
            .and_then(|uri| uri.to_file_path().ok());
        if let Some(path) = path {
            match crate::listing::describe_path(path.to_string_lossy().into_owned()) {
                Ok(entry) => entries.push(entry),
                Err(_) => skipped += 1,
            }
        } else {
            skipped += 1;
        }
    }
    Ok(
        json!({"entries": entries, "truncated": result.get("truncated").and_then(Value::as_bool).unwrap_or(false), "skipped": skipped}),
    )
}

#[tauri::command]
pub(crate) async fn linux_service(
    id: String,
    operation: String,
    payload: Value,
    requests: State<'_, ServiceRequests>,
    app: tauri::AppHandle,
) -> Result<Value, DirectoryError> {
    let cancelled = requests.begin(&id)?;
    let event_id = id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let indexed = operation == "indexed-search";
        let result = run(&operation, payload, cancelled, move |update| {
            let _ = app.emit(
                "linux-service-progress",
                json!({"id": event_id, "update": update}),
            );
        })?;
        if indexed {
            describe_indexed_results(result)
        } else {
            Ok(result)
        }
    })
    .await
    .map_err(|_| DirectoryError::operation_failed());
    requests.finish(&id);
    result?
}

#[tauri::command]
pub(crate) fn cancel_linux_service(id: String, requests: State<'_, ServiceRequests>) {
    if let Ok(requests) = requests.0.lock() {
        if let Some(cancel) = requests.get(&id) {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_ids_are_unique_and_cancellation_is_independent() {
        let requests = ServiceRequests::default();
        let first = requests.begin("pane-1").unwrap();
        let second = requests.begin("pane-2").unwrap();
        assert!(requests.begin("pane-1").is_err());
        assert!(requests.begin("../invalid").is_err());
        first.store(true, Ordering::Relaxed);
        assert!(!second.load(Ordering::Relaxed));
        requests.finish("pane-1");
        assert!(requests.begin("pane-1").is_ok());
    }

    #[test]
    fn unknown_operations_do_not_launch_a_process() {
        assert!(run("shell", json!({}), Arc::new(AtomicBool::new(false)), |_| {}).is_err());
    }
    #[test]
    fn indexed_results_skip_remote_and_stale_entries() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("matched.txt");
        std::fs::write(&path, "content").unwrap();
        let uri = url::Url::from_file_path(&path).unwrap().to_string();
        let result = describe_indexed_results(json!({"uris": [uri, "sftp://host/file", "file:///nonexistent-omafil-fixture"], "truncated": true})).unwrap();
        assert_eq!(result["entries"].as_array().unwrap().len(), 1);
        assert_eq!(result["entries"][0]["name"], "matched.txt");
        assert_eq!(result["skipped"], 2);
        assert_eq!(result["truncated"], true);
    }

    #[test]
    #[ignore = "requires optional system Python GObject bindings"]
    fn native_service_bridge_lists_real_files() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("bridge.txt"), "bridge fixture").unwrap();
        let result = run(
            "list",
            json!({"location": {"kind": "local", "path": directory.path()}}),
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .unwrap();
        assert_eq!(result["entries"][0]["name"], "bridge.txt");
    }
}
