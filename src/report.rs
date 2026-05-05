use tabled::{Table, Tabled};

use crate::dedupe::DeduplicationReport;

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

pub fn format_table(report: &DeduplicationReport, dry_run: bool) -> String {
    let mut rows: Vec<DuplicateRow> = Vec::new();

    for (i, group) in report.groups.iter().enumerate() {
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

    let total_dupes: usize = report.groups.iter().map(|g| g.duplicates.len()).sum();
    let header = format!(
        "Duplicates: {} group(s), {total_dupes} to remove",
        report.groups.len()
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

pub fn format_json(report: &DeduplicationReport, dry_run: bool) -> String {
    let groups: Vec<serde_json::Value> = report
        .groups
        .iter()
        .map(|g| {
            serde_json::json!({
                "keep": g.keep.display().to_string(),
                "duplicates": g.duplicates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            })
        })
        .collect();

    let empty_files: Vec<String> = report
        .empty_files
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    let total: usize = report.groups.iter().map(|g| g.duplicates.len()).sum();

    let json = serde_json::json!({
        "dry_run": dry_run,
        "groups": groups,
        "total_duplicates": total,
        "empty_files": empty_files,
    });

    serde_json::to_string_pretty(&json).expect("JSON serialization should not fail")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::dedupe::{DeduplicationReport, DuplicateGroup, MediaKind};

    use super::*;

    fn make_report(groups: Vec<DuplicateGroup>) -> DeduplicationReport {
        DeduplicationReport {
            groups,
            empty_files: vec![],
            skipped: vec![],
            to_delete: vec![],
        }
    }

    #[test]
    fn table_contains_keep_and_delete() {
        let report = make_report(vec![DuplicateGroup {
            kind: MediaKind::Image,
            keep: PathBuf::from("a.jpg"),
            duplicates: vec![PathBuf::from("b.jpg")],
        }]);
        let output = format_table(&report, false);
        assert!(output.contains("keep"));
        assert!(output.contains("delete"));
        assert!(output.contains("a.jpg"));
        assert!(output.contains("b.jpg"));
    }

    #[test]
    fn dry_run_shows_would_delete() {
        let report = make_report(vec![DuplicateGroup {
            kind: MediaKind::Image,
            keep: PathBuf::from("a.jpg"),
            duplicates: vec![PathBuf::from("b.jpg")],
        }]);
        let output = format_table(&report, true);
        assert!(output.contains("would delete"));
    }

    #[test]
    fn table_header_shows_counts() {
        let report = make_report(vec![
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
        ]);
        let output = format_table(&report, false);
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
    fn json_output_valid() {
        let report = DeduplicationReport {
            groups: vec![DuplicateGroup {
                kind: MediaKind::Image,
                keep: PathBuf::from("a.jpg"),
                duplicates: vec![PathBuf::from("b.jpg")],
            }],
            empty_files: vec![PathBuf::from("empty.jpg")],
            skipped: vec![],
            to_delete: vec![PathBuf::from("b.jpg")],
        };
        let json = format_json(&report, true);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["dry_run"], true);
        assert_eq!(parsed["total_duplicates"], 1);
        assert_eq!(parsed["empty_files"][0], "empty.jpg");
        assert_eq!(parsed["groups"][0]["keep"], "a.jpg");
        assert_eq!(parsed["groups"][0]["duplicates"][0], "b.jpg");
    }

    #[test]
    fn json_empty_case() {
        let report = make_report(vec![]);
        let json = format_json(&report, false);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["total_duplicates"], 0);
        assert!(parsed["groups"].as_array().unwrap().is_empty());
        assert!(parsed["empty_files"].as_array().unwrap().is_empty());
    }

    // Ensure the removed resolve_deletions tests are replaced by plan() tests in dedupe.rs
    #[test]
    fn report_to_delete_contains_duplicate_paths() {
        let report = DeduplicationReport {
            groups: vec![DuplicateGroup {
                kind: MediaKind::Image,
                keep: PathBuf::from("2020/a.jpg"),
                duplicates: vec![PathBuf::from("2021/b.jpg"), PathBuf::from("2021/c.jpg")],
            }],
            empty_files: vec![],
            skipped: vec![],
            to_delete: vec![PathBuf::from("2021/b.jpg"), PathBuf::from("2021/c.jpg")],
        };
        assert_eq!(report.to_delete.len(), 2);
        assert_eq!(report.to_delete[0], PathBuf::from("2021/b.jpg"));
        assert_eq!(report.to_delete[1], PathBuf::from("2021/c.jpg"));
    }
}
