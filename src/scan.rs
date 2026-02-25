use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use img_hash::ImageHash;
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

pub struct HashedFile {
    pub relative_path: String,
    pub hash: ImageHash,
}

pub struct DuplicateGroup {
    pub keep: String,
    pub duplicates: Vec<String>,
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

pub fn pairwise_compare(hashes: &[HashedFile], threshold: u32) -> HashMap<String, Vec<String>> {
    let mut duplicates: HashMap<String, Vec<String>> = HashMap::new();
    for h in hashes {
        duplicates.entry(h.relative_path.clone()).or_default();
    }

    for i in 0..hashes.len() {
        for j in (i + 1)..hashes.len() {
            let distance = hashes[i].hash.dist(&hashes[j].hash);
            if distance <= threshold {
                duplicates
                    .entry(hashes[i].relative_path.clone())
                    .or_default()
                    .push(hashes[j].relative_path.clone());
                duplicates
                    .entry(hashes[j].relative_path.clone())
                    .or_default()
                    .push(hashes[i].relative_path.clone());
            }
        }
    }

    duplicates
}

pub fn build_duplicate_groups(duplicates: &HashMap<String, Vec<String>>) -> Vec<DuplicateGroup> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut groups: Vec<DuplicateGroup> = Vec::new();

    let mut keys: Vec<&String> = duplicates.keys().collect();
    keys.sort();

    for filename in keys {
        let dupes = &duplicates[filename];
        if visited.contains(filename) || dupes.is_empty() {
            continue;
        }

        let mut group: HashSet<String> = HashSet::new();
        group.insert(filename.clone());
        let mut stack: Vec<String> = dupes.clone();

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

        let mut sorted: Vec<String> = group.into_iter().collect();
        sorted.sort();
        let keep = sorted.remove(0);
        groups.push(DuplicateGroup {
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

    fn make_hash(bytes: &[u8]) -> ImageHash {
        ImageHash::from_bytes(bytes).unwrap()
    }

    #[test]
    fn grouping_empty_input() {
        let duplicates = HashMap::new();
        let groups = build_duplicate_groups(&duplicates);
        assert!(groups.is_empty());
    }

    #[test]
    fn grouping_no_duplicates() {
        let mut duplicates = HashMap::new();
        duplicates.insert("a.jpg".to_string(), vec![]);
        duplicates.insert("b.jpg".to_string(), vec![]);
        let groups = build_duplicate_groups(&duplicates);
        assert!(groups.is_empty());
    }

    #[test]
    fn grouping_single_pair() {
        let mut duplicates = HashMap::new();
        duplicates.insert("a.jpg".to_string(), vec!["b.jpg".to_string()]);
        duplicates.insert("b.jpg".to_string(), vec!["a.jpg".to_string()]);
        let groups = build_duplicate_groups(&duplicates);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].keep, "a.jpg");
        assert_eq!(groups[0].duplicates, vec!["b.jpg"]);
    }

    #[test]
    fn grouping_transitive() {
        let mut duplicates = HashMap::new();
        duplicates.insert("a.jpg".to_string(), vec!["b.jpg".to_string()]);
        duplicates.insert(
            "b.jpg".to_string(),
            vec!["a.jpg".to_string(), "c.jpg".to_string()],
        );
        duplicates.insert("c.jpg".to_string(), vec!["b.jpg".to_string()]);
        let groups = build_duplicate_groups(&duplicates);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].keep, "a.jpg");
        assert_eq!(groups[0].duplicates, vec!["b.jpg", "c.jpg"]);
    }

    #[test]
    fn grouping_two_separate_groups() {
        let mut duplicates = HashMap::new();
        duplicates.insert("a.jpg".to_string(), vec!["b.jpg".to_string()]);
        duplicates.insert("b.jpg".to_string(), vec!["a.jpg".to_string()]);
        duplicates.insert("c.jpg".to_string(), vec!["d.jpg".to_string()]);
        duplicates.insert("d.jpg".to_string(), vec!["c.jpg".to_string()]);
        let groups = build_duplicate_groups(&duplicates);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].keep, "a.jpg");
        assert_eq!(groups[1].keep, "c.jpg");
    }

    #[test]
    fn grouping_keeps_first_alphabetically() {
        let mut duplicates = HashMap::new();
        duplicates.insert("z.jpg".to_string(), vec!["a.jpg".to_string()]);
        duplicates.insert("a.jpg".to_string(), vec!["z.jpg".to_string()]);
        let groups = build_duplicate_groups(&duplicates);
        assert_eq!(groups[0].keep, "a.jpg");
        assert_eq!(groups[0].duplicates, vec!["z.jpg"]);
    }

    #[test]
    fn pairwise_finds_duplicates() {
        let hash_a = make_hash(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let hash_b = make_hash(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let hash_c = make_hash(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);

        let hashes = vec![
            HashedFile {
                relative_path: "a.jpg".to_string(),
                hash: hash_a,
            },
            HashedFile {
                relative_path: "b.jpg".to_string(),
                hash: hash_b,
            },
            HashedFile {
                relative_path: "c.jpg".to_string(),
                hash: hash_c,
            },
        ];

        let result = pairwise_compare(&hashes, 1);
        assert_eq!(result["a.jpg"], vec!["b.jpg"]);
        assert_eq!(result["b.jpg"], vec!["a.jpg"]);
        assert!(result["c.jpg"].is_empty());
    }

    #[test]
    fn pairwise_respects_threshold() {
        let hash_a = make_hash(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let hash_b = make_hash(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let hashes = vec![
            HashedFile {
                relative_path: "a.jpg".to_string(),
                hash: hash_a.clone(),
            },
            HashedFile {
                relative_path: "b.jpg".to_string(),
                hash: hash_b,
            },
        ];

        let strict = pairwise_compare(&hashes, 0);
        assert!(strict["a.jpg"].is_empty());

        let lenient = pairwise_compare(&hashes, 1);
        assert_eq!(lenient["a.jpg"], vec!["b.jpg"]);
    }
}
