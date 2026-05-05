use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use walkdir::WalkDir;

use crate::dedupe::{DuplicateGroup, MediaKind};

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

pub fn collect_files(
    directories: &[PathBuf],
    extensions: &HashSet<&str>,
) -> eyre::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();

    for directory in directories {
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
    }

    files.sort();
    files.dedup();
    Ok(files)
}

pub fn build_duplicate_groups(
    duplicates: &HashMap<PathBuf, Vec<PathBuf>>,
    kind: MediaKind,
) -> Vec<DuplicateGroup> {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut groups: Vec<DuplicateGroup> = Vec::new();

    let mut keys: Vec<&PathBuf> = duplicates.keys().collect();
    keys.sort();

    for filename in keys {
        let dupes = &duplicates[filename];
        if visited.contains(filename) || dupes.is_empty() {
            continue;
        }

        let mut group: HashSet<PathBuf> = HashSet::new();
        group.insert(filename.clone());
        let mut stack: Vec<PathBuf> = dupes.clone();

        while let Some(current) = stack.pop() {
            if group.contains(&current) {
                continue;
            }
            group.insert(current.clone());
            if let Some(neighbors) = duplicates.get(&current) {
                for neighbor in neighbors {
                    if !group.contains(neighbor) {
                        stack.push(neighbor.clone());
                    }
                }
            }
        }

        visited.extend(group.iter().cloned());

        let mut sorted: Vec<PathBuf> = group.into_iter().collect();
        sorted.sort();
        let keep = sorted.remove(0);
        groups.push(DuplicateGroup {
            kind,
            keep,
            duplicates: sorted,
        });
    }

    groups
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn image_exts() -> HashSet<&'static str> {
        IMAGE_EXTENSIONS.iter().copied().collect()
    }

    fn dirs(d: &tempfile::TempDir) -> Vec<PathBuf> {
        vec![d.path().to_path_buf()]
    }

    #[test]
    fn empty_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let files = collect_files(&dirs(&dir), &image_exts()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn filters_by_extension() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), &[0xFF]).unwrap();
        fs::write(dir.path().join("b.txt"), &[0xFF]).unwrap();
        fs::write(dir.path().join("c.png"), &[0xFF]).unwrap();

        let files = collect_files(&dirs(&dir), &image_exts()).unwrap();
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

        let files = collect_files(&dirs(&dir), &image_exts()).unwrap();
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

        let files = collect_files(&dirs(&dir), &image_exts()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn results_are_sorted() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("c.jpg"), &[0xFF]).unwrap();
        fs::write(dir.path().join("a.jpg"), &[0xFF]).unwrap();
        fs::write(dir.path().join("b.jpg"), &[0xFF]).unwrap();

        let files = collect_files(&dirs(&dir), &image_exts()).unwrap();
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

        let files = collect_files(&dirs(&dir), &image_exts()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn multiple_directories_merged() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        fs::write(dir1.path().join("a.jpg"), &[0xFF]).unwrap();
        fs::write(dir2.path().join("b.jpg"), &[0xFF]).unwrap();

        let directories = vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()];
        let files = collect_files(&directories, &image_exts()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn deduplicates_overlapping_directories() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), &[0xFF]).unwrap();

        let directories = vec![dir.path().to_path_buf(), dir.path().to_path_buf()];
        let files = collect_files(&directories, &image_exts()).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn grouping_empty_input() {
        let duplicates = HashMap::new();
        let groups = build_duplicate_groups(&duplicates, MediaKind::Image);
        assert!(groups.is_empty());
    }

    #[test]
    fn grouping_no_duplicates() {
        let mut duplicates: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        duplicates.insert(PathBuf::from("a.jpg"), vec![]);
        duplicates.insert(PathBuf::from("b.jpg"), vec![]);
        let groups = build_duplicate_groups(&duplicates, MediaKind::Image);
        assert!(groups.is_empty());
    }

    #[test]
    fn grouping_single_pair() {
        let mut duplicates: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        duplicates.insert(PathBuf::from("a.jpg"), vec![PathBuf::from("b.jpg")]);
        duplicates.insert(PathBuf::from("b.jpg"), vec![PathBuf::from("a.jpg")]);
        let groups = build_duplicate_groups(&duplicates, MediaKind::Image);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].keep, PathBuf::from("a.jpg"));
        assert_eq!(groups[0].duplicates, vec![PathBuf::from("b.jpg")]);
    }

    #[test]
    fn grouping_transitive() {
        let mut duplicates: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        duplicates.insert(PathBuf::from("a.jpg"), vec![PathBuf::from("b.jpg")]);
        duplicates.insert(
            PathBuf::from("b.jpg"),
            vec![PathBuf::from("a.jpg"), PathBuf::from("c.jpg")],
        );
        duplicates.insert(PathBuf::from("c.jpg"), vec![PathBuf::from("b.jpg")]);
        let groups = build_duplicate_groups(&duplicates, MediaKind::Image);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].keep, PathBuf::from("a.jpg"));
        assert_eq!(
            groups[0].duplicates,
            vec![PathBuf::from("b.jpg"), PathBuf::from("c.jpg")]
        );
    }

    #[test]
    fn grouping_two_separate_groups() {
        let mut duplicates: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        duplicates.insert(PathBuf::from("a.jpg"), vec![PathBuf::from("b.jpg")]);
        duplicates.insert(PathBuf::from("b.jpg"), vec![PathBuf::from("a.jpg")]);
        duplicates.insert(PathBuf::from("c.jpg"), vec![PathBuf::from("d.jpg")]);
        duplicates.insert(PathBuf::from("d.jpg"), vec![PathBuf::from("c.jpg")]);
        let groups = build_duplicate_groups(&duplicates, MediaKind::Image);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].keep, PathBuf::from("a.jpg"));
        assert_eq!(groups[1].keep, PathBuf::from("c.jpg"));
    }

    #[test]
    fn grouping_keeps_first_alphabetically() {
        let mut duplicates: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        duplicates.insert(PathBuf::from("z.jpg"), vec![PathBuf::from("a.jpg")]);
        duplicates.insert(PathBuf::from("a.jpg"), vec![PathBuf::from("z.jpg")]);
        let groups = build_duplicate_groups(&duplicates, MediaKind::Image);
        assert_eq!(groups[0].keep, PathBuf::from("a.jpg"));
        assert_eq!(groups[0].duplicates, vec![PathBuf::from("z.jpg")]);
    }
}
