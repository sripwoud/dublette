pub mod cli;
pub mod dedupe;
pub mod delete;
pub mod hash;
pub mod report;
pub mod scan;

use std::path::PathBuf;

use cli::{Args, MediaFilter};
use dedupe::{Config, IndicatifProgress, MediaKind, NoopProgress};

pub fn run(args: &Args) -> eyre::Result<bool> {
    let config = Config {
        threshold: args.threshold,
        only: match &args.only {
            Some(MediaFilter::Images) => Some(MediaKind::Image),
            Some(MediaFilter::Videos) => Some(MediaKind::Video),
            None => None,
        },
        include_empty: args.delete_empty,
    };

    let progress: Box<dyn dedupe::Progress> = if args.quiet {
        Box::new(NoopProgress)
    } else {
        Box::new(IndicatifProgress::new(args.verbose))
    };

    let dedup_report = dedupe::plan(&args.directories, &config, progress.as_ref())?;

    for skipped in &dedup_report.skipped {
        eprintln!(
            "Warning: skipping {}: {}",
            skipped.path.display(),
            skipped.reason
        );
    }

    let found_duplicates = !dedup_report.groups.is_empty();

    if args.json {
        println!("{}", report::format_json(&dedup_report, args.dry_run));
    } else {
        if !dedup_report.empty_files.is_empty() {
            println!(
                "{}",
                report::format_empty_table(&dedup_report.empty_files, args.dry_run)
            );
        }
        if found_duplicates {
            println!("{}", report::format_table(&dedup_report, args.dry_run));
        } else if dedup_report.empty_files.is_empty() {
            println!("No duplicates found.");
        }
    }

    let total_deleted = if args.dry_run {
        0
    } else {
        let mut count = 0usize;
        if !dedup_report.empty_files.is_empty() {
            count += delete::delete_files(&dedup_report.empty_files, "empty", args.yes)?;
        }
        if found_duplicates {
            let dup_paths: Vec<PathBuf> = dedup_report
                .groups
                .iter()
                .flat_map(|g| g.duplicates.iter().cloned())
                .collect();
            count += delete::delete_files(&dup_paths, "duplicate", args.yes)?;
        }
        count
    };

    if !args.json {
        if args.dry_run && found_duplicates {
            let total: usize = dedup_report.groups.iter().map(|g| g.duplicates.len()).sum();
            println!("\n[dry run] {} file(s) would be deleted.", total);
        } else if total_deleted > 0 {
            eprintln!("\nRemoved {total_deleted} duplicate(s) total.");
        }
    }

    Ok(found_duplicates)
}
