use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::scan;

pub fn delete_files(
    paths: &[PathBuf],
    directory: &Path,
    label: &str,
    yes: bool,
) -> eyre::Result<usize> {
    if paths.is_empty() {
        return Ok(0);
    }

    if !yes {
        let prompt = format!("Delete {} {label} file(s)?", paths.len());
        if !dialoguer::Confirm::new().with_prompt(prompt).interact()? {
            return Ok(0);
        }
    }

    let mut deleted = 0;
    for path in paths {
        fs::remove_file(path)?;
        if let Ok(rel) = path.strip_prefix(directory) {
            eprintln!("  Deleted: {}", rel.display());
        }
        deleted += 1;
    }

    Ok(deleted)
}

pub fn find_empty_files(directory: &Path) -> eyre::Result<Vec<PathBuf>> {
    let media_exts = scan::all_media_extensions();
    let mut empty: Vec<PathBuf> = Vec::new();

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

        if !media_exts.contains(ext.as_str()) {
            continue;
        }

        if entry.metadata()?.len() == 0 {
            empty.push(path.to_path_buf());
        }
    }

    empty.sort();
    Ok(empty)
}

pub fn collect_zero_byte_files(
    directory: &Path,
    extensions: &HashSet<&str>,
) -> eyre::Result<Vec<PathBuf>> {
    let mut empty: Vec<PathBuf> = Vec::new();

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
            empty.push(path.to_path_buf());
        }
    }

    empty.sort();
    Ok(empty)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn find_empty_media_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("empty.jpg"), &[]).unwrap();
        fs::write(dir.path().join("valid.jpg"), &[0xFF]).unwrap();
        fs::write(dir.path().join("empty.txt"), &[]).unwrap();

        let empty = find_empty_files(dir.path()).unwrap();
        assert_eq!(empty.len(), 1);
        assert!(empty[0].file_name().unwrap().to_str().unwrap() == "empty.jpg");
    }

    #[test]
    fn delete_files_removes_them() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.jpg");
        let b = dir.path().join("b.jpg");
        fs::write(&a, &[0xFF]).unwrap();
        fs::write(&b, &[0xFF]).unwrap();

        let deleted = delete_files(&[a.clone()], dir.path(), "image", true).unwrap();
        assert_eq!(deleted, 1);
        assert!(!a.exists());
        assert!(b.exists());
    }

    #[test]
    fn delete_files_empty_list_returns_zero() {
        let dir = tempfile::tempdir().unwrap();
        let deleted = delete_files(&[], dir.path(), "image", true).unwrap();
        assert_eq!(deleted, 0);
    }
}
