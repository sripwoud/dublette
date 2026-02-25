use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "bmp", "gif", "tiff", "webp"];

pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v", "3gp",
];

pub fn all_media_extensions() -> HashSet<&'static str> {
    IMAGE_EXTENSIONS
        .iter()
        .chain(VIDEO_EXTENSIONS.iter())
        .copied()
        .collect()
}

pub fn collect_files(directory: &Path, extensions: &HashSet<&str>) -> eyre::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(directory) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e.to_lowercase(),
            None => continue,
        };

        if !extensions.contains(ext.as_str()) {
            continue;
        }

        if entry.metadata()?.len() == 0 {
            continue;
        }

        files.push(path.to_path_buf());
    }

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn image_exts() -> HashSet<&'static str> {
        IMAGE_EXTENSIONS.iter().copied().collect()
    }

    #[test]
    fn empty_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let files = collect_files(dir.path(), &image_exts()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn filters_by_extension() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), &[0xFF]).unwrap();
        fs::write(dir.path().join("b.txt"), &[0xFF]).unwrap();
        fs::write(dir.path().join("c.png"), &[0xFF]).unwrap();

        let files = collect_files(dir.path(), &image_exts()).unwrap();
        let names: Vec<&str> = files
            .iter()
            .filter_map(|p| p.file_name()?.to_str())
            .collect();
        assert_eq!(names, vec!["a.jpg", "c.png"]);
    }

    #[test]
    fn excludes_zero_byte_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("empty.jpg"), &[]).unwrap();
        fs::write(dir.path().join("valid.jpg"), &[0xFF]).unwrap();

        let files = collect_files(dir.path(), &image_exts()).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().unwrap().to_str().unwrap() == "valid.jpg");
    }

    #[test]
    fn recurses_subdirectories() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(dir.path().join("a.jpg"), &[0xFF]).unwrap();
        fs::write(sub.join("b.jpg"), &[0xFF]).unwrap();

        let files = collect_files(dir.path(), &image_exts()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn results_are_sorted() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("c.jpg"), &[0xFF]).unwrap();
        fs::write(dir.path().join("a.jpg"), &[0xFF]).unwrap();
        fs::write(dir.path().join("b.jpg"), &[0xFF]).unwrap();

        let files = collect_files(dir.path(), &image_exts()).unwrap();
        let names: Vec<&str> = files
            .iter()
            .filter_map(|p| p.file_name()?.to_str())
            .collect();
        assert_eq!(names, vec!["a.jpg", "b.jpg", "c.jpg"]);
    }

    #[test]
    fn case_insensitive_extensions() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.JPG"), &[0xFF]).unwrap();
        fs::write(dir.path().join("b.Png"), &[0xFF]).unwrap();

        let files = collect_files(dir.path(), &image_exts()).unwrap();
        assert_eq!(files.len(), 2);
    }
}
