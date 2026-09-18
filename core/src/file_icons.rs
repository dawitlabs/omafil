//! Freedesktop icon names for files and folders, so every front end resolves
//! the same icon for the same entry.

const GENERIC: &str = "text-x-generic";

/// Standard folders get their own icon; everything else is a plain folder.
pub fn folder_icon_name(folder_name: &str) -> &'static str {
    match folder_name.to_ascii_lowercase().as_str() {
        "documents" => "folder-documents",
        "downloads" => "folder-download",
        "music" => "folder-music",
        "pictures" => "folder-pictures",
        "videos" => "folder-videos",
        "desktop" => "folder-desktop",
        _ => "folder",
    }
}

/// The icon for a file, chosen by extension.
///
/// A name with no extension, and a dotfile with nothing after its dot, both
/// fall back to the generic file icon — not to a folder, which is what the
/// Svelte build does for these.
pub fn file_icon_name(name: &str) -> &'static str {
    let Some(dot) = name.rfind('.') else { return GENERIC };
    if dot == 0 {
        return GENERIC;
    }
    match name[dot + 1..].to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "bmp" | "svg" | "ico" => "image-x-generic",
        "mp3" | "wav" | "flac" | "m4a" | "ogg" | "opus" | "aac" => "audio-x-generic",
        "mp4" | "mkv" | "mov" | "webm" | "avi" | "m4v" => "video-x-generic",
        "doc" | "docx" | "odt" | "rtf" => "x-office-document",
        "xls" | "xlsx" | "ods" | "csv" | "tsv" => "x-office-spreadsheet",
        "ppt" | "pptx" | "odp" => "x-office-presentation",
        "pdf" => "application-pdf",
        "zip" | "tar" | "gz" | "xz" | "zst" | "7z" | "rar" => "package-x-generic",
        "ts" | "tsx" | "js" | "jsx" | "svelte" | "vue" | "rs" | "py" | "go" | "c" | "h" | "cpp"
        | "java" | "rb" | "sh" | "json" | "toml" | "yaml" | "yml" | "xml" | "xaml" | "html" | "css" => "text-x-script",
        "txt" | "md" | "log" => GENERIC,
        _ => GENERIC,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_choose_the_icon_and_case_does_not_matter() {
        assert_eq!(file_icon_name("holiday.JPEG"), "image-x-generic");
        assert_eq!(file_icon_name("report.pdf"), "application-pdf");
        assert_eq!(file_icon_name("main.rs"), "text-x-script");
        assert_eq!(file_icon_name("archive.tar.gz"), "package-x-generic");
        assert_eq!(file_icon_name("notes.unknown-extension"), GENERIC);
    }

    #[test]
    fn a_file_without_a_usable_extension_is_never_a_folder() {
        for name in ["Makefile", "LICENSE", ".bashrc", "trailing."] {
            assert_eq!(file_icon_name(name), GENERIC, "{name} should read as a file");
        }
    }

    #[test]
    fn standard_folders_keep_their_own_icons() {
        assert_eq!(folder_icon_name("Downloads"), "folder-download");
        assert_eq!(folder_icon_name("pictures"), "folder-pictures");
        assert_eq!(folder_icon_name("Code"), "folder");
    }
}
