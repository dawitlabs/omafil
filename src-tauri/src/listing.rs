use crate::error::DirectoryError;
use crate::paths::{display_name, navigable_roots, resolve_navigable_path};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub(crate) const MAX_PAGE_SIZE: usize = 1_000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryEntry {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) entry_type: DirectoryEntryType,
    pub(crate) size: u64,
    pub(crate) modified: Option<i64>,
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

impl EntrySort {
    /// Size and date order the whole directory by fields only a stat call can
    /// supply, so those two columns have to stat every entry before paging.
    /// Name and type read straight off the directory, so they stat one page.
    fn needs_metadata(self) -> bool {
        matches!(self, EntrySort::Size | EntrySort::Modified)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PathCrumb {
    pub(crate) name: String,
    pub(crate) path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryListing {
    pub(crate) path: String,
    pub(crate) crumbs: Vec<PathCrumb>,
    pub(crate) entries: Vec<DirectoryEntry>,
    pub(crate) total: usize,
    pub(crate) has_more: bool,
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

fn name_order(left: &str, right: &str) -> Ordering {
    left.to_lowercase().cmp(&right.to_lowercase())
}

fn extension_order(left: &str, right: &str) -> Ordering {
    entry_extension(left).cmp(&entry_extension(right))
}

/// Directories lead whatever the column, and the direction only ever flips the
/// column itself, so every ordering in this module ends here.
fn order_by(
    left_is_directory: bool,
    right_is_directory: bool,
    by_column: Ordering,
    descending: bool,
) -> Ordering {
    right_is_directory
        .cmp(&left_is_directory)
        .then(if descending { by_column.reverse() } else { by_column })
}

pub(crate) fn compare_entries(
    left: &DirectoryEntry,
    right: &DirectoryEntry,
    sort: EntrySort,
    descending: bool,
) -> Ordering {
    let by_name = name_order(&left.name, &right.name);

    let by_column = match sort {
        EntrySort::Name => by_name,
        EntrySort::Size => left.size.cmp(&right.size).then(by_name),
        EntrySort::Modified => left.modified.cmp(&right.modified).then(by_name),
        EntrySort::Type => extension_order(&left.name, &right.name).then(by_name),
    };

    order_by(
        left.entry_type == DirectoryEntryType::Directory,
        right.entry_type == DirectoryEntryType::Directory,
        by_column,
        descending,
    )
}

fn compare_unstated(
    left: &UnstatedEntry,
    right: &UnstatedEntry,
    sort: EntrySort,
    descending: bool,
) -> Ordering {
    debug_assert!(!sort.needs_metadata(), "size and date cannot order unstated entries");
    let by_name = name_order(&left.name, &right.name);

    let by_column = match sort {
        EntrySort::Type => extension_order(&left.name, &right.name).then(by_name),
        _ => by_name,
    };

    order_by(
        left.entry_type == DirectoryEntryType::Directory,
        right.entry_type == DirectoryEntryType::Directory,
        by_column,
        descending,
    )
}

impl DirectoryEntry {
    #[cfg(test)]
    pub(crate) fn path(&self) -> &str {
        &self.path
    }
}

/// A directory entry before its stat call. `read_dir` hands over the name and
/// the kind for free; size and modified time are the two fields that cost a
/// syscall each, which is why they are filled in only for entries being sent.
struct UnstatedEntry {
    name: String,
    path: PathBuf,
    entry_type: DirectoryEntryType,
}

impl UnstatedEntry {
    fn read(directory_entry: &fs::DirEntry) -> Option<Self> {
        Some(UnstatedEntry {
            name: directory_entry.file_name().to_string_lossy().into_owned(),
            path: directory_entry.path(),
            entry_type: if directory_entry.file_type().ok()?.is_dir() {
                DirectoryEntryType::Directory
            } else {
                DirectoryEntryType::File
            },
        })
    }

    /// `fs::DirEntry::metadata` does not follow symlinks, so neither does this.
    fn stat(self) -> DirectoryEntry {
        let metadata = self.path.symlink_metadata().ok();

        DirectoryEntry {
            name: self.name,
            path: self.path.to_string_lossy().into_owned(),
            entry_type: self.entry_type,
            size: metadata.as_ref().map_or(0, |data| data.len()),
            modified: metadata
                .as_ref()
                .and_then(|data| data.modified().ok())
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|elapsed| elapsed.as_secs() as i64),
        }
    }
}

pub(crate) fn describe_entry(directory_entry: &fs::DirEntry) -> Option<DirectoryEntry> {
    Some(UnstatedEntry::read(directory_entry)?.stat())
}

/// Type-ahead matches the way a reader scanning the folder would: anywhere in
/// the name, either case. An empty filter keeps everything.
fn matches_filter(name: &str, filter: &str) -> bool {
    filter.is_empty() || name.to_lowercase().contains(&filter.to_lowercase())
}

fn read_unstated_entries(
    directory: &Path,
    show_hidden: bool,
    filter: &str,
) -> std::io::Result<Vec<UnstatedEntry>> {
    let mut entries = Vec::new();

    for directory_entry in fs::read_dir(directory)? {
        let directory_entry = directory_entry?;

        let Some(entry) = UnstatedEntry::read(&directory_entry) else {
            continue;
        };

        let is_hidden = entry.name.starts_with('.');

        if (show_hidden || !is_hidden) && matches_filter(&entry.name, filter) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn page<T>(entries: Vec<T>, offset: usize, page_size: usize) -> Vec<T> {
    entries.into_iter().skip(offset).take(page_size).collect()
}

/// Returns one page of matching entries and how many matched, statting every
/// entry only when the sort column needs it. See [`EntrySort::needs_metadata`].
#[cfg(test)]
pub(crate) fn read_directory_page(
    directory: &Path,
    sort: EntrySort,
    descending: bool,
    show_hidden: bool,
    offset: usize,
    page_size: usize,
    filter: &str,
) -> std::io::Result<(Vec<DirectoryEntry>, usize)> {
    read_directory_page_revealing(directory, sort, descending, show_hidden, offset, page_size, filter, &[])
}

fn read_directory_page_revealing(
    directory: &Path, sort: EntrySort, descending: bool, show_hidden: bool,
    offset: usize, page_size: usize, filter: &str, reveal_paths: &[String],
) -> std::io::Result<(Vec<DirectoryEntry>, usize)> {
    let requested_paths: std::collections::HashSet<&Path> = reveal_paths.iter().map(Path::new).collect();
    let requested = |path: &Path| requested_paths.contains(path);
    let mut unstated = read_unstated_entries(directory, show_hidden || !reveal_paths.is_empty(), filter)?;
    if !show_hidden {
        unstated.retain(|entry| !entry.name.starts_with('.') || requested(&entry.path));
    }
    let total = unstated.len();

    if sort.needs_metadata() {
        let mut entries: Vec<DirectoryEntry> = unstated.into_iter().map(UnstatedEntry::stat).collect();
        entries.sort_by(|left, right| requested(Path::new(&right.path)).cmp(&requested(Path::new(&left.path)))
            .then_with(|| compare_entries(left, right, sort, descending)));

        return Ok((page(entries, offset, page_size), total));
    }

    unstated.sort_by(|left, right| requested(&right.path).cmp(&requested(&left.path))
        .then_with(|| compare_unstated(left, right, sort, descending)));

    let entries = page(unstated, offset, page_size)
        .into_iter()
        .map(UnstatedEntry::stat)
        .collect();

    Ok((entries, total))
}

/// The sidebar tree only needs the folders, and only their names, so it never pays
/// for the stat calls or the pagination that a full listing carries.
pub(crate) fn read_subdirectories(path: &str, show_hidden: bool) -> Result<Vec<PathCrumb>, DirectoryError> {
    let directory = resolve_navigable_path(path)?;
    let mut folders: Vec<PathCrumb> = fs::read_dir(&directory)
        .map_err(DirectoryError::from)?
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| PathCrumb {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
        })
        .filter(|folder| show_hidden || !folder.name.starts_with('.'))
        .collect();

    folders.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    Ok(folders)
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
        .map_err(DirectoryError::from)?;

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

#[cfg(test)]
pub(crate) fn read_directory_listing(
    path: String,
    sort: EntrySort,
    descending: bool,
    show_hidden: bool,
    offset: usize,
    limit: usize,
    filter: &str,
) -> Result<DirectoryListing, DirectoryError> {
    read_directory_listing_revealing(path, sort, descending, show_hidden, offset, limit, filter, &[])
}

pub(crate) fn read_directory_listing_revealing(
    path: String, sort: EntrySort, descending: bool, show_hidden: bool,
    offset: usize, limit: usize, filter: &str, reveal_paths: &[String],
) -> Result<DirectoryListing, DirectoryError> {
    if reveal_paths.len() > crate::desktop_requests::MAX_TARGETS {
        return Err(DirectoryError::detail("Too many items to reveal."));
    }
    let directory = resolve_navigable_path(&path)?;

    if !directory.is_dir() {
        return Err(DirectoryError::unavailable());
    }

    let page_size = limit.clamp(1, MAX_PAGE_SIZE);
    let (entries, total) =
        read_directory_page_revealing(&directory, sort, descending, show_hidden, offset, page_size, filter, reveal_paths)
            .map_err(DirectoryError::from)?;
    let has_more = offset.saturating_add(page_size) < total;

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
    #[test]
    fn reveals_hidden_and_later_items_without_breaking_pagination() {
        use super::*;
        let root = tempfile::tempdir().unwrap();
        for name in ["a", "b", "c", "z", ".requested", ".hidden"] {
            std::fs::write(root.path().join(name), name).unwrap();
        }
        let reveal = vec![root.path().join("z").to_str().unwrap().to_owned(), root.path().join(".requested").to_str().unwrap().to_owned()];
        for sort in [EntrySort::Name, EntrySort::Size, EntrySort::Modified, EntrySort::Type] {
            let (first, total) = read_directory_page_revealing(root.path(), sort, false, false, 0, 2, "", &reveal).unwrap();
            assert_eq!(total, 5);
            assert!(first.iter().all(|entry| reveal.contains(&entry.path)));
            let (rest, _) = read_directory_page_revealing(root.path(), sort, false, false, 2, 10, "", &reveal).unwrap();
            assert_eq!(rest.len(), 3);
            assert!(rest.iter().all(|entry| !reveal.contains(&entry.path)));
        }
        let (_, total) = read_directory_page(root.path(), EntrySort::Name, false, false, 0, 10, "").unwrap();
        assert_eq!(total, 4);
    }

    use super::{
        compare_entries, compare_unstated, entry_extension, matches_filter, path_crumbs,
        read_directory_page, DirectoryEntry, DirectoryEntryType, EntrySort, UnstatedEntry,
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
    fn system_breadcrumbs_reach_filesystem_root() {
        let crumbs = path_crumbs(Path::new("/etc/ssh"), &crate::paths::navigable_roots());
        assert_eq!(crumbs.iter().map(|crumb| crumb.path.as_str()).collect::<Vec<_>>(), ["/", "/etc", "/etc/ssh"]);
    }

    #[test]
    fn denied_listing_reports_permissions_instead_of_empty_results() {
        use std::{fs, os::unix::fs::PermissionsExt};
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0)).unwrap();
        let result = super::read_directory_listing(root.path().to_string_lossy().into_owned(), EntrySort::Name, false, false, 0, 100, "");
        // Restore even if the assertion fails so fixture cleanup stays possible.
        let denied = fs::read_dir(root.path()).is_err();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        if denied {
            let error = result.err().expect("denied listing must fail");
            assert_eq!(serde_json::to_value(error).unwrap()["code"], "permission_denied");
        }
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

    #[test]
    fn only_size_and_date_need_the_stat_calls() {
        assert!(EntrySort::Size.needs_metadata());
        assert!(EntrySort::Modified.needs_metadata());
        assert!(!EntrySort::Name.needs_metadata());
        assert!(!EntrySort::Type.needs_metadata());
    }

    /// The cheap path has to reach the same order as the full one, or paging by
    /// name would silently return different entries than it used to.
    #[test]
    fn the_unstated_order_matches_the_statted_one() {
        let names = ["Work", "a.txt", "B.md", "archive.tar.gz", ".hidden", "README"];
        let listing = || {
            names
                .iter()
                .enumerate()
                .map(|(index, name)| entry(name, *name == "Work", index as u64))
                .collect::<Vec<_>>()
        };
        let unstated = || {
            names
                .iter()
                .map(|name| UnstatedEntry {
                    name: (*name).to_owned(),
                    path: PathBuf::from(format!("/home/dave/{name}")),
                    entry_type: if *name == "Work" {
                        DirectoryEntryType::Directory
                    } else {
                        DirectoryEntryType::File
                    },
                })
                .collect::<Vec<_>>()
        };

        for sort in [EntrySort::Name, EntrySort::Type] {
            for descending in [false, true] {
                let mut cheap = unstated();
                cheap.sort_by(|left, right| compare_unstated(left, right, sort, descending));

                assert_eq!(
                    cheap.into_iter().map(|item| item.name).collect::<Vec<_>>(),
                    sorted(listing(), sort, descending),
                );
            }
        }
    }

    #[test]
    fn a_page_carries_the_real_sizes_and_the_full_total() {
        let directory = tempfile::tempdir().unwrap();
        for (name, bytes) in [("a.txt", 1_usize), ("b.txt", 22), ("c.txt", 333)] {
            std::fs::write(directory.path().join(name), vec![b'x'; bytes]).unwrap();
        }

        let (entries, total) =
            read_directory_page(directory.path(), EntrySort::Name, false, false, 1, 1, "").unwrap();

        assert_eq!(total, 3);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "b.txt");
        assert_eq!(entries[0].size, 22);
    }

    #[test]
    fn the_filter_matches_anywhere_in_the_name_either_case() {
        assert!(matches_filter("Q3-Report.pdf", "report"));
        assert!(matches_filter("q3-report.pdf", "REPORT"));
        assert!(matches_filter("anything", ""));
        assert!(!matches_filter("notes.md", "report"));
    }

    /// The total drives "showing X of Y" and the load-more button, so it has to
    /// count matches rather than the whole directory.
    #[test]
    fn a_filtered_total_counts_only_the_matches() {
        let directory = tempfile::tempdir().unwrap();
        for name in ["q3-report.pdf", "q4-report.pdf", "notes.md"] {
            std::fs::write(directory.path().join(name), b"x").unwrap();
        }

        let (entries, total) =
            read_directory_page(directory.path(), EntrySort::Name, false, false, 0, 10, "REPORT")
                .unwrap();

        assert_eq!(total, 2);
        assert_eq!(
            entries.into_iter().map(|item| item.name).collect::<Vec<_>>(),
            ["q3-report.pdf", "q4-report.pdf"],
        );
    }

    #[test]
    fn hidden_entries_stay_out_of_the_total() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("visible.txt"), b"x").unwrap();
        std::fs::write(directory.path().join(".hidden"), b"x").unwrap();

        let (_, shown) =
            read_directory_page(directory.path(), EntrySort::Name, false, false, 0, 10, "").unwrap();
        let (_, all) =
            read_directory_page(directory.path(), EntrySort::Name, false, true, 0, 10, "").unwrap();

        assert_eq!((shown, all), (1, 2));
    }
}
