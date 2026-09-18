use crate::error::DirectoryError;
use crate::listing::{describe_entry, DirectoryEntry};
use crate::paths::resolve_navigable_path;
use serde::Serialize;
use std::{
    collections::VecDeque,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

const MAX_SEARCH_RESULTS: usize = 500;
const MAX_SEARCH_VISITS: usize = 60_000;
/// How often the walk checks whether a newer search has replaced it.
const CANCEL_CHECK_EVERY: usize = 256;
/// Below this, a subsequence match is noise: "ab" hits almost every name.
const MIN_FUZZY_QUERY: usize = 3;

#[derive(Default)]
struct SearchFilter {
    terms: Vec<String>,
    extensions: Vec<String>,
    minimum_size: Option<u64>,
    maximum_size: Option<u64>,
}

fn parse_size(value: &str) -> Option<u64> {
    let value = value.trim().to_lowercase();
    let (number, multiplier) = if let Some(number) = value.strip_suffix("kb") { (number, 1024) }
    else if let Some(number) = value.strip_suffix("mb") { (number, 1024 * 1024) }
    else if let Some(number) = value.strip_suffix("gb") { (number, 1024 * 1024 * 1024) }
    else { (value.as_str(), 1) };
    number.trim().parse::<u64>().ok()?.checked_mul(multiplier)
}

fn type_extensions(value: &str) -> Vec<String> {
    match value {
        "image" | "images" => ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif"].into_iter().map(str::to_owned).collect(),
        "video" | "videos" => ["mp4", "mkv", "webm", "avi", "mov", "mpeg"].into_iter().map(str::to_owned).collect(),
        "audio" | "music" => ["mp3", "wav", "flac", "ogg", "m4a", "aac"].into_iter().map(str::to_owned).collect(),
        "document" | "documents" => ["pdf", "doc", "docx", "odt", "txt", "md", "rtf"].into_iter().map(str::to_owned).collect(),
        "archive" | "archives" => ["zip", "7z", "tar", "gz", "xz", "bz2", "rar"].into_iter().map(str::to_owned).collect(),
        other => vec![other.trim_start_matches('.').to_owned()],
    }
}

fn parse_filter(query: &str) -> SearchFilter {
    let mut filter = SearchFilter::default();
    for token in query.split_whitespace() {
        if let Some(value) = token.strip_prefix("type:").or_else(|| token.strip_prefix("ext:")) {
            filter.extensions.extend(type_extensions(&value.to_lowercase()));
        } else if let Some(value) = token.strip_prefix("size:>") {
            filter.minimum_size = parse_size(value);
        } else if let Some(value) = token.strip_prefix("size:<") {
            filter.maximum_size = parse_size(value);
        } else {
            filter.terms.push(token.to_owned());
        }
    }
    filter
}

fn matches_filter(filter: &SearchFilter, name: &str, size: u64) -> bool {
    let extension = name.rsplit_once('.').map(|(_, extension)| extension.to_lowercase()).unwrap_or_default();
    (filter.extensions.is_empty() || filter.extensions.iter().any(|wanted| wanted == &extension))
        && filter.minimum_size.is_none_or(|minimum| size > minimum)
        && filter.maximum_size.is_none_or(|maximum| size < maximum)
}

#[derive(Default)]
pub struct SearchGeneration(Arc<AtomicU64>);

impl SearchGeneration {
    pub fn begin(&self) -> (Arc<AtomicU64>, u64) {
        let current = Arc::clone(&self.0);
        let generation = current.fetch_add(1, Ordering::SeqCst) + 1;

        (current, generation)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub entries: Vec<DirectoryEntry>,
    pub truncated: bool,
    pub skipped: usize,
}

/// Case-insensitive glob over a single name. Supports `*` and `?` only, which
/// is what a file manager's search box is asked for in practice.
pub fn matches_glob(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();
    let (mut p, mut n) = (0, 0);
    let (mut star, mut resume) = (None, 0);

    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            resume = n;
            p += 1;
        } else if let Some(position) = star {
            // Backtrack: let the last star swallow one more character.
            p = position + 1;
            resume += 1;
            n = resume;
        } else {
            return false;
        }
    }

    pattern[p..].iter().all(|token| *token == '*')
}

fn is_subsequence(query: &str, name: &str) -> bool {
    let mut characters = name.chars();

    query.chars().all(|wanted| characters.any(|found| found == wanted))
}

/// Higher is a better match. `None` means the name does not match at all.
pub fn match_score(query: &str, name: &str) -> Option<u32> {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);

    if name == query {
        return Some(100);
    }

    if stem == query {
        return Some(90);
    }

    if name.starts_with(query) {
        return Some(80);
    }

    if let Some(position) = name.find(query) {
        let preceded_by_boundary = name[..position]
            .chars()
            .last()
            .is_some_and(|character| !character.is_alphanumeric());

        return Some(if preceded_by_boundary { 60 } else { 40 });
    }

    if query.chars().count() >= MIN_FUZZY_QUERY && is_subsequence(query, name) {
        return Some(20);
    }

    None
}

pub fn search_directory(
    path: String,
    query: String,
    show_hidden: bool,
    generation: u64,
    current: Arc<AtomicU64>,
) -> Result<SearchResults, DirectoryError> {
    let root = resolve_navigable_path(&path)?;
    // A failed root must be an error, not a misleading empty search.
    fs::read_dir(&root)?;

    Ok(walk_matches(root, &query, show_hidden, generation, current))
}

pub fn walk_matches(
    root: PathBuf,
    query: &str,
    show_hidden: bool,
    generation: u64,
    current: Arc<AtomicU64>,
) -> SearchResults {
    let filter = parse_filter(query);
    let needle = filter.terms.join(" ").to_lowercase();

    if needle.is_empty() && filter.extensions.is_empty() && filter.minimum_size.is_none() && filter.maximum_size.is_none() {
        return SearchResults {
            entries: Vec::new(),
            truncated: false,
            skipped: 0,
        };
    }

    let is_glob = needle.contains('*') || needle.contains('?');
    // Breadth first, so the nearest matches are found before the result budget
    // is spent somewhere deep in one branch.
    let mut pending: VecDeque<(PathBuf, usize)> = VecDeque::from([(root, 0)]);
    let mut matches: Vec<(u32, usize, usize, String, DirectoryEntry)> = Vec::new();
    let mut visits = 0_usize;
    let mut truncated = false;
    let mut skipped = 0;

    'walk: while let Some((directory, depth)) = pending.pop_front() {
        if current.load(Ordering::SeqCst) != generation {
            return SearchResults {
                entries: Vec::new(),
                truncated: false,
                skipped: 0,
            };
        }

        let Ok(directory_entries) = fs::read_dir(&directory) else {
            skipped += 1;
            continue;
        };

        for directory_entry in directory_entries {
            let Ok(directory_entry) = directory_entry else {
                skipped += 1;
                continue;
            };
            visits += 1;

            if visits.is_multiple_of(CANCEL_CHECK_EVERY) && current.load(Ordering::SeqCst) != generation {
                return SearchResults {
                    entries: Vec::new(),
                    truncated: false,
                    skipped: 0,
                };
            }

            if visits > MAX_SEARCH_VISITS || matches.len() >= MAX_SEARCH_RESULTS {
                truncated = true;
                break 'walk;
            }

            let name = directory_entry.file_name().to_string_lossy().into_owned();

            if !show_hidden && name.starts_with('.') {
                continue;
            }

            // Filters constrain results, never traversal. Otherwise type:image
            // prunes every ordinary parent folder before reaching its images.
            match directory_entry.file_type() {
                Ok(kind) if kind.is_dir() => pending.push_back((directory_entry.path(), depth + 1)),
                Ok(_) => {}, // Do not descend into symlinks (including cycles).
                Err(_) => { skipped += 1; continue; }
            }
            let size = match directory_entry.metadata() {
                Ok(metadata) => metadata.len(),
                Err(_) => { skipped += 1; continue; }
            };
            if !matches_filter(&filter, &name, size) {
                continue;
            }

            let folded = name.to_lowercase();
            let score = if needle.is_empty() {
                Some(50)
            } else if is_glob {
                matches_glob(&needle, &folded).then_some(50)
            } else {
                match_score(&needle, &folded)
            };

            if let Some(score) = score {
                if let Some(entry) = describe_entry(&directory_entry) {
                    matches.push((score, depth, name.chars().count(), folded, entry));
                }
            }
        }
    }

    // Best match first, then nearest, then the shortest name, which is the one
    // most likely to be what was meant.
    matches.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then(left.1.cmp(&right.1))
            .then(left.2.cmp(&right.2))
            .then_with(|| left.3.cmp(&right.3))
    });

    SearchResults {
        entries: matches.into_iter().map(|(.., entry)| entry).collect(),
        truncated,
        skipped,
    }
}

#[cfg(test)]
mod tests {
    use super::{match_score, matches_filter, matches_glob, parse_filter, walk_matches};
    use std::{
        fs,
        path::PathBuf,
        sync::{atomic::AtomicU64, Arc},
    };

    fn tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("omafil-search-{name}-{}", std::process::id()));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(root.join("deep/one/two/three")).unwrap();
        fs::write(root.join("deep/one/two/three/target.txt"), b"deep").unwrap();
        fs::write(root.join("target.txt"), b"shallow").unwrap();
        fs::write(root.join("untargeted-notes.txt"), b"other").unwrap();
        root
    }

    fn search(root: &PathBuf, query: &str) -> Vec<String> {
        walk_matches(root.clone(), query, false, 1, Arc::new(AtomicU64::new(1)))
            .entries
            .into_iter()
            .map(|entry| entry.path().to_owned())
            .collect()
    }

    #[test]
    fn search_marks_inaccessible_subfolders_and_preserves_visible_matches() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let denied = root.path().join("private");
        fs::create_dir(&denied).unwrap();
        fs::write(root.path().join("visible.txt"), b"visible").unwrap();
        fs::set_permissions(&denied, fs::Permissions::from_mode(0)).unwrap();
        let is_denied = fs::read_dir(&denied).is_err();
        let results = walk_matches(root.path().to_path_buf(), "*.txt", false, 1, Arc::new(AtomicU64::new(1)));
        fs::set_permissions(&denied, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(results.entries.len(), 1);
        if is_denied { assert_eq!(results.skipped, 1); }
    }

    #[test]
    fn filtered_search_respects_hidden_folders_and_does_not_follow_cycles() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".hidden")).unwrap();
        fs::write(root.path().join(".hidden/secret.jpg"), b"image").unwrap();
        std::os::unix::fs::symlink(root.path(), root.path().join("cycle")).unwrap();
        assert!(search(&root.path().to_path_buf(), "type:image").is_empty());
        let results = walk_matches(root.path().to_path_buf(), "type:image", true, 1, Arc::new(AtomicU64::new(1)));
        assert_eq!(results.entries.len(), 1);
        assert!(!results.truncated);
    }

    #[test]
    fn type_filter_reaches_files_in_unmatched_subdirectories() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("photos/summer")).unwrap();
        fs::write(root.path().join("photos/summer/beach.jpg"), b"image").unwrap();
        let found = search(&root.path().to_path_buf(), "type:image");
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("beach.jpg"));
    }

    #[test]
    fn size_filter_does_not_prune_small_parent_directories() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("nested")).unwrap();
        let file = fs::File::create(root.path().join("nested/large.bin")).unwrap();
        file.set_len(2 * 1024 * 1024).unwrap();
        let found = search(&root.path().to_path_buf(), "size:>1mb");
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("large.bin"));
    }

    #[test]
    fn globs_cover_the_shapes_a_search_box_gets() {
        assert!(matches_glob("*.pdf", "report.pdf"));
        assert!(matches_glob("report*", "report-final.docx"));
        assert!(matches_glob("?ata.csv", "data.csv"));
        assert!(matches_glob("*", "anything"));
        assert!(matches_glob("*rep*rt*", "my-report-2026.txt"));

        assert!(!matches_glob("*.pdf", "report.pdf.bak"));
        assert!(!matches_glob("?ata.csv", "metadata.csv"));
        assert!(!matches_glob("report*", "final-report.docx"));
    }

    #[test]
    fn a_trailing_star_still_needs_the_prefix() {
        assert!(matches_glob("a*b*", "axxbyy"));
        assert!(!matches_glob("a*b*", "xxbyy"));
    }

    #[test]
    fn filters_by_common_type_and_size() {
        let filter = parse_filter("type:image size:>1mb size:<5mb vacation");

        assert_eq!(filter.terms, ["vacation"]);
        assert!(matches_filter(&filter, "photo.JPG", 2 * 1024 * 1024));
        assert!(!matches_filter(&filter, "photo.JPG", 6 * 1024 * 1024));
        assert!(!matches_filter(&filter, "notes.txt", 2 * 1024 * 1024));
    }

    #[test]
    fn better_matches_score_higher() {
        let exact = match_score("notes", "notes").unwrap();
        let stem = match_score("notes", "notes.txt").unwrap();
        let prefix = match_score("notes", "notes-2026.txt").unwrap();
        let boundary = match_score("notes", "my-notes.txt").unwrap();
        let substring = match_score("notes", "mynotes.txt").unwrap();

        assert!(exact > stem);
        assert!(stem > prefix);
        assert!(prefix > boundary);
        assert!(boundary > substring);
    }

    #[test]
    fn a_nearby_match_outranks_an_identical_one_buried_deeper() {
        let root = tree("depth");
        let found = search(&root, "target.txt");

        // The shallow exact match, then the identical one four levels down,
        // then untargeted-notes.txt, which only matches as a subsequence.
        assert_eq!(found.len(), 3);
        assert!(found[0].ends_with("/target.txt") && !found[0].contains("/deep/"));
        assert!(found[1].contains("/deep/one/two/three/"));
        assert!(found[2].ends_with("/untargeted-notes.txt"));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_superseded_search_stops_and_returns_nothing() {
        let root = tree("cancel");
        let current = Arc::new(AtomicU64::new(9));
        let results = walk_matches(root.clone(), "target", false, 1, current);

        assert!(results.entries.is_empty());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn fuzzy_matching_needs_a_long_enough_query() {
        assert_eq!(match_score("ab", "a-big-file.txt"), None);
        assert!(match_score("abf", "a-big-file.txt").is_some());
        assert_eq!(match_score("zzz", "a-big-file.txt"), None);
    }
}
