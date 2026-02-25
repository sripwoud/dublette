use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Clone, ValueEnum)]
pub enum MediaFilter {
    Images,
    Videos,
}

#[derive(Parser)]
#[command(
    name = "imgdedup",
    about = "Deduplicate images and videos using perceptual hashing"
)]
pub struct Args {
    #[arg(help = "Directory to scan for duplicates")]
    pub directory: PathBuf,

    #[arg(
        short,
        long,
        default_value_t = 1,
        help = "Max hamming distance to consider as duplicate"
    )]
    pub threshold: u32,

    #[arg(short = 'n', long, help = "List duplicates without deleting")]
    pub dry_run: bool,

    #[arg(long, value_enum, help = "Process only images or only videos")]
    pub only: Option<MediaFilter>,

    #[arg(long, help = "Delete 0-byte media files")]
    pub delete_empty: bool,

    #[arg(short, long, help = "Skip confirmation prompt")]
    pub yes: bool,

    #[arg(short, long, help = "Suppress progress output")]
    pub quiet: bool,

    #[arg(short, long, help = "Show verbose output")]
    pub verbose: bool,

    #[arg(long, help = "Disable color output")]
    pub no_color: bool,

    #[arg(long, help = "Output results as JSON")]
    pub json: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Args {
        Args::parse_from(args)
    }

    #[test]
    fn defaults() {
        let args = parse(&["imgdedup", "/tmp"]);
        assert_eq!(args.threshold, 1);
        assert!(!args.dry_run);
        assert!(!args.delete_empty);
        assert!(!args.yes);
        assert!(!args.quiet);
        assert!(!args.verbose);
        assert!(!args.no_color);
        assert!(!args.json);
        assert!(args.only.is_none());
    }

    #[test]
    fn all_flags() {
        let args = parse(&[
            "imgdedup",
            "/tmp",
            "-n",
            "--delete-empty",
            "-y",
            "-q",
            "-v",
            "--no-color",
            "--json",
            "-t",
            "5",
            "--only",
            "images",
        ]);
        assert_eq!(args.threshold, 5);
        assert!(args.dry_run);
        assert!(args.delete_empty);
        assert!(args.yes);
        assert!(args.quiet);
        assert!(args.verbose);
        assert!(args.no_color);
        assert!(args.json);
        assert!(matches!(args.only, Some(MediaFilter::Images)));
    }

    #[test]
    fn only_videos() {
        let args = parse(&["imgdedup", "/tmp", "--only", "videos"]);
        assert!(matches!(args.only, Some(MediaFilter::Videos)));
    }

    #[test]
    fn missing_directory_fails() {
        let result = Args::try_parse_from(&["imgdedup"]);
        assert!(result.is_err());
    }
}
