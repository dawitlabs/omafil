use crate::watcher::start_watch;
use notify::RecommendedWatcher;
use std::{collections::HashMap, fs, path::PathBuf};

/// `omarchy theme set` rebuilds this directory, so its parent is what gets watched.
fn current_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state/omarchy/current"))
}

pub type ThemeColors = HashMap<String, String>;

/// colors.toml is a flat list of `key = "value"` lines, so a full TOML parser
/// is not needed.
pub fn parse_colors(source: &str) -> ThemeColors {
    source
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let value = value.trim().trim_matches('"');

            if value.is_empty() || key.trim().is_empty() {
                return None;
            }

            Some((key.trim().to_owned(), value.to_owned()))
        })
        .collect()
}

pub fn read_theme() -> Option<ThemeColors> {
    let source = fs::read_to_string(current_dir()?.join("theme/colors.toml")).ok()?;
    let colors = parse_colors(&source);

    colors.contains_key("background").then_some(colors)
}

pub struct ThemeWatcher(#[allow(dead_code)] RecommendedWatcher);

pub fn watch_theme<F>(on_change: F) -> Option<ThemeWatcher>
where
    F: Fn() + Send + 'static,
{
    let directory = current_dir().filter(|dir| dir.is_dir())?;

    start_watch(directory, move |_| on_change()).ok().map(ThemeWatcher)
}

#[cfg(test)]
mod tests {
    use super::parse_colors;

    #[test]
    fn parses_flat_colors_and_skips_junk() {
        let colors = parse_colors("mode = \"dark\"\n\naccent = \"#f38d70\"\n# comment\nbroken\nempty = \"\"\n");

        assert_eq!(colors.get("mode").map(String::as_str), Some("dark"));
        assert_eq!(colors.get("accent").map(String::as_str), Some("#f38d70"));
        assert_eq!(colors.len(), 2);
    }
}
