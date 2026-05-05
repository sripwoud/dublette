use std::path::PathBuf;

use serde::Serialize;
use tabled::{Table, Tabled};

use crate::dedupe::DuplicateGroup;

#[derive(Serialize)]
pub struct JsonReport {
    pub empty_files: Vec<String>,
    pub groups: Vec<JsonGroup>,
    pub total_duplicates: usize,
    pub dry_run: bool,
}

#[derive(Serialize)]
pub struct JsonGroup {
    pub keep: String,
    pub duplicates: Vec<String>,
}

#[derive(Tabled)]
struct DuplicateRow {
    #[tabled(rename = "Group")]
    group: String,
    #[tabled(rename = "File")]
    file: String,
    #[tabled(rename = "Action")]
    action: String,
}

#[derive(Tabled)]
struct EmptyRow {
    #[tabled(rename = "File")]
    file: String,
    #[tabled(rename = "Action")]
    action: String,
}

pub fn format_table(groups: &[DuplicateGroup], dry_run: bool, label: &str) -> String {
    let mut rows: Vec<DuplicateRow> = Vec::new();

    for (i, group) in groups.iter().enumerate() {
        rows.push(DuplicateRow {
            group: (i + 1).to_string(),
            file: group.keep.display().to_string(),
            action: "keep".to_string(),
        });
        for dup in &group.duplicates {
            rows.push(DuplicateRow {
                group: String::new(),
                file: dup.display().to_string(),
                action: if dry_run {
                    "would delete".to_string()
                } else {
                    "delete".to_string()
                },
            });
        }
    }

    let total_dupes: usize = groups.iter().map(|g| g.duplicates.len()).sum();
    let header = format!(
        "Duplicate {label}s: {} group(s), {total_dupes} to remove",
        groups.len()
    );

    format!("{header}\n{}", Table::new(rows))
}

pub fn format_empty_table(empty_files: &[String], dry_run: bool) -> String {
    let rows: Vec<EmptyRow> = empty_files
        .iter()
        .map(|f| EmptyRow {
            file: f.clone(),
            action: if dry_run {
                "would delete".to_string()
            } else {
                "delete".to_string()
            },
        })
        .collect();

    let header = format!("Empty (0-byte) files ({})", empty_files.len());
    format!("{header}\n{}", Table::new(rows))
}

pub fn format_json(groups: &[DuplicateGroup], empty_files: &[String], dry_run: bool) -> String {
    let json_groups: Vec<JsonGroup> = groups
        .iter()
        .map(|g| JsonGroup {
            keep: g.keep.display().to_string(),
            duplicates: g
                .duplicates
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
        })
        .collect();
    let total: usize = groups.iter().map(|g| g.duplicates.len()).sum();
    let report = JsonReport {
        empty_files: empty_files.to_vec(),
        groups: json_groups,
        total_duplicates: total,
        dry_run,
    };
    serde_json::to_string_pretty(&report).expect("JSON serialization should not fail")
}

pub fn resolve_deletions(groups: &[DuplicateGroup]) -> Vec<PathBuf> {
    groups
        .iter()
        .flat_map(|g| g.duplicates.iter().cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::dedupe::MediaKind;

    use super::*;

    #[test]
    fn table_contains_keep_and_delete() {
        let groups = vec![DuplicateGroup {
            kind: MediaKind::Image,
            keep: PathBuf::from("a.jpg"),
            duplicates: vec![PathBuf::from("b.jpg")],
        }];
        let output = format_table(&groups, false, "image");
        assert!(output.contains("keep"));
        assert!(output.contains("delete"));
        assert!(output.contains("a.jpg"));
        assert!(output.contains("b.jpg"));
    }

    #[test]
    fn dry_run_shows_would_delete() {
        let groups = vec![DuplicateGroup {
            kind: MediaKind::Image,
            keep: PathBuf::from("a.jpg"),
            duplicates: vec![PathBuf::from("b.jpg")],
        }];
        let output = format_table(&groups, true, "image");
        assert!(output.contains("would delete"));
    }

    #[test]
    fn table_header_shows_counts() {
        let groups = vec![
            DuplicateGroup {
                kind: MediaKind::Image,
                keep: PathBuf::from("a.jpg"),
                duplicates: vec![PathBuf::from("b.jpg"), PathBuf::from("c.jpg")],
            },
            DuplicateGroup {
                kind: MediaKind::Image,
                keep: PathBuf::from("d.jpg"),
                duplicates: vec![PathBuf::from("e.jpg")],
            },
        ];
        let output = format_table(&groups, false, "image");
        assert!(output.contains("2 group(s)"));
        assert!(output.contains("3 to remove"));
    }

    #[test]
    fn empty_table_shows_count() {
        let files = vec!["a.jpg".to_string(), "b.jpg".to_string()];
        let output = format_empty_table(&files, true);
        assert!(output.contains("2"));
        assert!(output.contains("would delete"));
    }

    #[test]
    fn resolve_deletions_returns_duplicate_paths() {
        let groups = vec![DuplicateGroup {
            kind: MediaKind::Image,
            keep: PathBuf::from("2020/a.jpg"),
            duplicates: vec![PathBuf::from("2021/b.jpg"), PathBuf::from("2021/c.jpg")],
        }];
        let paths = resolve_deletions(&groups);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("2021/b.jpg"));
        assert_eq!(paths[1], PathBuf::from("2021/c.jpg"));
    }

    #[test]
    fn json_output_valid() {
        let groups = vec![DuplicateGroup {
            kind: MediaKind::Image,
            keep: PathBuf::from("a.jpg"),
            duplicates: vec![PathBuf::from("b.jpg")],
        }];
        let json = format_json(&groups, &["empty.jpg".to_string()], true);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["dry_run"], true);
        assert_eq!(parsed["total_duplicates"], 1);
        assert_eq!(parsed["empty_files"][0], "empty.jpg");
        assert_eq!(parsed["groups"][0]["keep"], "a.jpg");
        assert_eq!(parsed["groups"][0]["duplicates"][0], "b.jpg");
    }

    #[test]
    fn json_empty_case() {
        let json = format_json(&[], &[], false);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["total_duplicates"], 0);
        assert!(parsed["groups"].as_array().unwrap().is_empty());
        assert!(parsed["empty_files"].as_array().unwrap().is_empty());
    }
}
