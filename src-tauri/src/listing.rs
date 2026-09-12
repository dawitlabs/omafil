use crate::error::DirectoryError;
use crate::paths::{display_name, navigable_roots, resolve_navigable_path};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub(crate) const MAX_PAGE_SIZE: usize = 1_000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryEntry {
    name: String,
    path: String,
    entry_type: DirectoryEntryType,
    size: u64,
    modified: Option<i64>,
}

#[derive(PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DirectoryEntryType {
    Directory,
    File,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum EntrySort {
    Name,
    Size,
    Modified,
    Type,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PathCrumb {
    name: String,
    path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryListing {
    path: String,
    crumbs: Vec<PathCrumb>,
    entries: Vec<DirectoryEntry>,
    total: usize,
    has_more: bool,
}

pub(crate) fn entry_extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(stem, extension)| {
            if stem.is_empty() {
                String::new()
            } else {
                extension.to_lowercase()
            }
        })
        .unwrap_or_default()
}

pub(crate) fn compare_entries(
    left: &DirectoryEntry,
    right: &DirectoryEntry,
    sort: EntrySort,
    descending: bool,
) -> std::cmp::Ordering {
    let left_is_directory = left.entry_type == DirectoryEntryType::Directory;
    let right_is_directory = right.entry_type == DirectoryEntryType::Directory;
    let by_name = left.name.to_lowercase().cmp(&right.name.to_lowercase());

    let by_key = match sort {
        EntrySort::Name => by_name,
        EntrySort::Size => left.size.cmp(&right.size).then(by_name),
        EntrySort::Modified => left.modified.cmp(&right.modified).then(by_name),
        EntrySort::Type => entry_extension(&left.name)
            .cmp(&entry_extension(&right.name))
            .then(by_name),
    };

    right_is_directory
        .cmp(&left_is_directory)
        .then(if descending { by_key.reverse() } else { by_key })
}

impl DirectoryEntry {
    #[cfg(test)]
    pub(crate) fn path(&self) -> &str {
        &self.path
    }
}

pub(crate) fn describe_entry(directory_entry: &fs::DirEntry) -> Option<DirectoryEntry> {
    let file_type = directory_entry.file_type().ok()?;
    let metadata = directory_entry.metadata().ok();

    Some(DirectoryEntry {
        name: directory_entry.file_name().to_string_lossy().into_owned(),
        path: directory_entry.path().to_string_lossy().into_owned(),
        entry_type: if file_type.is_dir() {
            DirectoryEntryType::Directory
        } else {
            DirectoryEntryType::File
        },
        size: metadata.as_ref().map_or(0, |data| data.len()),
        modified: metadata
            .as_ref()
            .and_then(|data| data.modified().ok())
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|elapsed| elapsed.as_secs() as i64),
    })
}

pub(crate) fn read_directory_entries(
    directory: &Path,
    sort: EntrySort,
    descending: bool,
    show_hidden: bool,
) -> Result<(Vec<DirectoryEntry>, usize), ()> {
    let directory_entries = fs::read_dir(directory).map_err(|_| ())?;
    let mut entries = Vec::new();

    for directory_entry in directory_entries {
        let directory_entry = directory_entry.map_err(|_| ())?;
        let name = directory_entry.file_name().to_string_lossy().into_owned();

        if !show_hidden && name.starts_with('.') {
            continue;
        }

        if let Some(entry) = describe_entry(&directory_entry) {
            entries.push(entry);
        }
    }

    entries.sort_by(|left, right| compare_entries(left, right, sort, descending));

    let total = entries.len();
    Ok((entries, total))
}

pub(crate) fn path_crumbs(directory: &Path, roots: &[PathBuf]) -> Vec<PathCrumb> {
    let Some(root) = roots
        .iter()
        .filter(|root| directory.starts_with(root))
        .max_by_key(|root| root.components().count())
    else {
        return Vec::new();
    };

    let mut crumbs = vec![PathCrumb {
        name: display_name(root),
        path: root.to_string_lossy().into_owned(),
    }];
    let mut walked = root.to_path_buf();

    for component in directory.strip_prefix(root).unwrap_or(Path::new("")) {
        walked.push(component);
        crumbs.push(PathCrumb {
            name: component.to_string_lossy().into_owned(),
            path: walked.to_string_lossy().into_owned(),
        });
    }

    crumbs
}

pub(crate) fn describe_path(path: String) -> Result<DirectoryEntry, DirectoryError> {
    let target = resolve_navigable_path(&path)?;
    let metadata = target
        .symlink_metadata()
        .map_err(|_| DirectoryError::unavailable())?;

    Ok(DirectoryEntry {
        name: display_name(&target),
        path: target.to_string_lossy().into_owned(),
        entry_type: if metadata.is_dir() {
            DirectoryEntryType::Directory
        } else {
            DirectoryEntryType::File
        },
        size: metadata.len(),
        modified: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|elapsed| elapsed.as_secs() as i64),
    })
}

pub(crate) fn read_directory_listing(
    path: String,
    sort: EntrySort,
    descending: bool,
    show_hidden: bool,
    offset: usize,
    limit: usize,
) -> Result<DirectoryListing, DirectoryError> {
    let directory = resolve_navigable_path(&path)?;

    if !directory.is_dir() {
        return Err(DirectoryError::unavailable());
    }

    let (entries, total) = read_directory_entries(&directory, sort, descending, show_hidden)
        .map_err(|_| DirectoryError::read_failed())?;

    let page_size = limit.clamp(1, MAX_PAGE_SIZE);
    let has_more = offset.saturating_add(page_size) < total;
    let entries = entries.into_iter().skip(offset).take(page_size).collect();

    Ok(DirectoryListing {
        crumbs: path_crumbs(&directory, &navigable_roots()),
        path: directory.to_string_lossy().into_owned(),
        entries,
        total,
        has_more,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        compare_entries, entry_extension, path_crumbs, DirectoryEntry, DirectoryEntryType,
        EntrySort,
    };
    use std::path::{Path, PathBuf};

    fn entry(name: &str, is_directory: bool, size: u64) -> DirectoryEntry {
        DirectoryEntry {
            name: name.to_owned(),
            path: format!("/home/dave/{name}"),
            entry_type: if is_directory {
                DirectoryEntryType::Directory
            } else {
                DirectoryEntryType::File
            },
            size,
            modified: Some(size as i64),
        }
    }

    fn sorted(mut entries: Vec<DirectoryEntry>, sort: EntrySort, descending: bool) -> Vec<String> {
        entries.sort_by(|left, right| compare_entries(left, right, sort, descending));
        entries.into_iter().map(|item| item.name).collect()
    }

    #[test]
    fn crumbs_start_at_the_deepest_matching_root() {
        let roots = vec![PathBuf::from("/home/dave"), PathBuf::from("/run/media/usb")];
        let crumbs = path_crumbs(Path::new("/home/dave/Code/omafil"), &roots);

        let trail: Vec<_> = crumbs.iter().map(|crumb| crumb.name.as_str()).collect();
        assert_eq!(trail, ["dave", "Code", "omafil"]);
        assert_eq!(crumbs.last().unwrap().path, "/home/dave/Code/omafil");
    }

    #[test]
    fn crumbs_are_empty_outside_every_root() {
        let roots = vec![PathBuf::from("/home/dave")];

        assert!(path_crumbs(Path::new("/etc/ssh"), &roots).is_empty());
    }

    #[test]
    fn folders_stay_first_in_both_sort_directions() {
        let listing = || {
            vec![
                entry("big.iso", false, 900),
                entry("a.txt", false, 10),
                entry("Work", true, 0),
            ]
        };

        assert_eq!(
            sorted(listing(), EntrySort::Size, false),
            ["Work", "a.txt", "big.iso"]
        );
        assert_eq!(
            sorted(listing(), EntrySort::Size, true),
            ["Work", "big.iso", "a.txt"]
        );
        assert_eq!(
            sorted(listing(), EntrySort::Name, true),
            ["Work", "big.iso", "a.txt"]
        );
    }

    #[test]
    fn extensions_ignore_dotfiles_and_bare_names() {
        assert_eq!(entry_extension("notes.MD"), "md");
        assert_eq!(entry_extension("archive.tar.gz"), "gz");
        assert_eq!(entry_extension("README"), "");
        assert_eq!(entry_extension(".bashrc"), "");
    }
}
