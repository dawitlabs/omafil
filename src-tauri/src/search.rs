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
pub(crate) struct SearchGeneration(Arc<AtomicU64>);

impl SearchGeneration {
    pub(crate) fn begin(&self) -> (Arc<AtomicU64>, u64) {
        let current = Arc::clone(&self.0);
        let generation = current.fetch_add(1, Ordering::SeqCst) + 1;

        (current, generation)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResults {
    pub(crate) entries: Vec<DirectoryEntry>,
    pub(crate) truncated: bool,
}

/// Case-insensitive glob over a single name. Supports `*` and `?` only, which
/// is what a file manager's search box is asked for in practice.
pub(crate) fn matches_glob(pattern: &str, name: &str) -> bool {
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
pub(crate) fn match_score(query: &str, name: &str) -> Option<u32> {
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

pub(crate) fn search_directory(
    path: String,
    query: String,
    show_hidden: bool,
    generation: u64,
    current: Arc<AtomicU64>,
) -> Result<SearchResults, DirectoryError> {
    let root = resolve_navigable_path(&path)?;

    Ok(walk_matches(root, &query, show_hidden, generation, current))
}

pub(crate) fn walk_matches(
    root: PathBuf,
    query: &str,
    show_hidden: bool,
    generation: u64,
    current: Arc<AtomicU64>,
) -> SearchResults {
    let needle = query.trim().to_lowercase();

    if needle.is_empty() {
        return SearchResults {
            entries: Vec::new(),
            truncated: false,
        };
    }

    let is_glob = needle.contains('*') || needle.contains('?');
    // Breadth first, so the nearest matches are found before the result budget
    // is spent somewhere deep in one branch.
    let mut pending: VecDeque<(PathBuf, usize)> = VecDeque::from([(root, 0)]);
    let mut matches: Vec<(u32, usize, usize, String, DirectoryEntry)> = Vec::new();
    let mut visits = 0_usize;
    let mut truncated = false;

    'walk: while let Some((directory, depth)) = pending.pop_front() {
        if current.load(Ordering::SeqCst) != generation {
            return SearchResults {
                entries: Vec::new(),
                truncated: false,
            };
        }

        let Ok(directory_entries) = fs::read_dir(&directory) else {
            continue;
        };

        for directory_entry in directory_entries.flatten() {
            visits += 1;

            if visits.is_multiple_of(CANCEL_CHECK_EVERY) && current.load(Ordering::SeqCst) != generation {
                return SearchResults {
                    entries: Vec::new(),
                    truncated: false,
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

            // A symlinked directory is not descended into, so a cycle cannot trap the walk.
            if directory_entry
                .file_type()
                .is_ok_and(|file_type| file_type.is_dir())
            {
                pending.push_back((directory_entry.path(), depth + 1));
            }

            let folded = name.to_lowercase();
            let score = if is_glob {
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
    }
}

#[cfg(test)]
mod tests {
    use super::{match_score, matches_glob, walk_matches};
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
