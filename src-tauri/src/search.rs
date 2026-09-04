use crate::error::DirectoryError;
use crate::listing::{describe_entry, DirectoryEntry};
use crate::paths::resolve_navigable_path;
use serde::Serialize;
use std::{fs, path::PathBuf};

const MAX_SEARCH_RESULTS: usize = 500;
const MAX_SEARCH_VISITS: usize = 60_000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResults {
    pub(crate) entries: Vec<DirectoryEntry>,
    pub(crate) truncated: bool,
}

pub(crate) fn search_directory(
    path: String,
    query: String,
    show_hidden: bool,
) -> Result<SearchResults, DirectoryError> {
    let root = resolve_navigable_path(&path)?;
    let needle = query.trim().to_lowercase();

    if needle.is_empty() {
        return Ok(SearchResults {
            entries: Vec::new(),
            truncated: false,
        });
    }

    let mut pending: Vec<PathBuf> = vec![root];
    let mut entries = Vec::new();
    let mut visits = 0_usize;

    while let Some(directory) = pending.pop() {
        let Ok(directory_entries) = fs::read_dir(&directory) else {
            continue;
        };

        for directory_entry in directory_entries.flatten() {
            visits += 1;

            if visits > MAX_SEARCH_VISITS || entries.len() >= MAX_SEARCH_RESULTS {
                return Ok(SearchResults {
                    entries,
                    truncated: true,
                });
            }

            let name = directory_entry.file_name().to_string_lossy().into_owned();

            if !show_hidden && name.starts_with('.') {
                continue;
            }

            // A symlinked directory is not descended into, so a cycle cannot trap the walk.
            if directory_entry
                .file_type()
                .is_ok_and(|file_type| file_type.is_dir())
            {
                pending.push(directory_entry.path());
            }

            if name.to_lowercase().contains(&needle) {
                if let Some(entry) = describe_entry(&directory_entry) {
                    entries.push(entry);
                }
            }
        }
    }

    Ok(SearchResults {
        entries,
        truncated: false,
    })
}
