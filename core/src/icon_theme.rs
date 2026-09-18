use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

/// Icon names the UI asks for, in freedesktop naming-spec terms.
const NAMES: &[&str] = &[
    "folder", "folder-documents", "folder-download", "folder-music", "folder-pictures", "folder-videos", "folder-desktop",
    "text-x-generic", "image-x-generic", "audio-x-generic", "video-x-generic", "application-pdf", "package-x-generic",
    "text-x-script", "x-office-document", "x-office-spreadsheet", "x-office-presentation", "text-html",
];
const SIZES: &[&str] = &["48x48", "64x64", "32x32", "128x128", "256x256", "24x24", "22x22", "16x16", "scalable"];
const CATEGORIES: &[&str] = &["places", "mimetypes"];

pub fn current_theme_name() -> Option<String> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let name = fs::read_to_string(home.join(".local/state/omarchy/current/theme/icons.theme")).ok()?;
    let name = name.trim();

    (!name.is_empty()).then(|| name.to_owned())
}

fn icon_bases() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let data_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());

    home.iter()
        .flat_map(|home| [home.join(".local/share/icons"), home.join(".icons")])
        .chain(data_dirs.split(':').filter(|d| !d.is_empty()).map(|d| PathBuf::from(d).join("icons")))
        .collect()
}

fn parse_inherits(index: &str) -> Vec<String> {
    index
        .lines()
        .find_map(|line| line.trim().strip_prefix("Inherits="))
        .map(|value| value.split(',').map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned).collect())
        .unwrap_or_default()
}

/// Theme, its parents in order, then hicolor as the spec's final fallback.
fn theme_chain(name: &str, bases: &[PathBuf]) -> Vec<String> {
    let mut chain = Vec::new();
    let mut seen = HashSet::new();
    let mut queue = vec![name.to_owned()];

    while let Some(theme) = queue.pop() {
        if !seen.insert(theme.clone()) {
            continue;
        }
        let parents = bases
            .iter()
            .find_map(|base| fs::read_to_string(base.join(&theme).join("index.theme")).ok())
            .map(|index| parse_inherits(&index))
            .unwrap_or_default();
        chain.push(theme);
        for parent in parents.into_iter().rev() {
            queue.push(parent);
        }
    }
    if !seen.contains("hicolor") {
        chain.push("hicolor".to_owned());
    }

    chain
}

/// Yaru and Papirus lay icons out as `48x48/mimetypes`, Breeze as `mimetypes/48`.
fn lookup(name: &str, chain: &[String], bases: &[PathBuf]) -> Option<PathBuf> {
    for theme in chain {
        for base in bases {
            let root = base.join(theme);
            if !root.is_dir() {
                continue;
            }
            for size in SIZES {
                let short = size.split('x').next().unwrap_or(size);
                for category in CATEGORIES {
                    for extension in ["png", "svg"] {
                        let file = format!("{name}.{extension}");
                        let candidates = [root.join(size).join(category).join(&file), root.join(category).join(short).join(&file)];
                        if let Some(found) = candidates.into_iter().find(|p| p.is_file()) {
                            return Some(found);
                        }
                    }
                }
            }
        }
    }

    None
}

fn resolve_all(name: &str, bases: &[PathBuf]) -> HashMap<String, String> {
    let chain = theme_chain(name, bases);

    NAMES
        .iter()
        .filter_map(|icon| lookup(icon, &chain, bases).map(|path| ((*icon).to_owned(), path.to_string_lossy().into_owned())))
        .collect()
}

pub fn theme_icons() -> Option<HashMap<String, String>> {
    let name = current_theme_name()?;
    let icons = resolve_all(&name, &icon_bases());

    (!icons.is_empty()).then_some(icons)
}

#[allow(dead_code)]
fn touch(path: &Path) {
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, b"").expect("file");
}

#[cfg(test)]
mod tests {
    use super::{parse_inherits, resolve_all, touch};

    #[test]
    fn walks_inherited_themes_and_both_layouts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().to_path_buf();
        std::fs::create_dir_all(base.join("Child")).expect("theme dir");
        std::fs::write(base.join("Child/index.theme"), "[Icon Theme]\nName=Child\nInherits=Parent,hicolor\n").expect("index");
        touch(&base.join("Child/48x48/places/folder.png"));
        touch(&base.join("Parent/mimetypes/48/application-pdf.svg"));
        touch(&base.join("hicolor/16x16/mimetypes/text-html.png"));

        let icons = resolve_all("Child", &[base.clone()]);

        assert_eq!(parse_inherits("Inherits=A, B\n"), vec!["A", "B"]);
        assert!(icons["folder"].ends_with("Child/48x48/places/folder.png"));
        assert!(icons["application-pdf"].ends_with("Parent/mimetypes/48/application-pdf.svg"));
        assert!(icons["text-html"].ends_with("hicolor/16x16/mimetypes/text-html.png"));
        assert!(!icons.contains_key("audio-x-generic"));
    }
}
