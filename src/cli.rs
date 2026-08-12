use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::audio::MatchStrategy;

#[derive(Clone, ValueEnum)]
pub enum MediaFilter {
    Images,
    Videos,
    Audio,
}

#[derive(Parser)]
#[command(
    name = "dublette",
    version,
    about = "Deduplicate images, videos, and audio using perceptual hashing and acoustic fingerprints"
)]
pub struct Args {
    #[arg(help = "Directories to scan for duplicates", num_args = 1.., required = true)]
    pub directories: Vec<PathBuf>,

    #[arg(
        short,
        long,
        default_value_t = 1,
        help = "Max hamming distance to consider as duplicate"
    )]
    pub threshold: u32,

    #[arg(short = 'n', long, help = "List duplicates without deleting")]
    pub dry_run: bool,

    #[arg(long, value_enum, help = "Process only images, videos, or audio")]
    pub only: Option<MediaFilter>,

    #[arg(
        long,
        value_enum,
        default_value = "recording",
        help = "How audio files are matched: same recording (acoustic fingerprint) or same encoding (exact stream, tags excluded)"
    )]
    pub audio_match: MatchStrategy,

    #[arg(
        long,
        value_parser = parse_audio_threshold,
        help = "Max acoustic fingerprint dissimilarity (0.0-1.0) to consider as duplicate; recording match only [default: 0.1]"
    )]
    pub audio_threshold: Option<f64>,

    #[arg(
        long,
        value_name = "DIR",
        help = "Prefer keeping the group member inside this directory; repeatable, order = precedence"
    )]
    pub keep_in: Vec<PathBuf>,

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

fn parse_audio_threshold(value: &str) -> Result<f64, String> {
    let threshold: f64 = value.parse().map_err(|e| format!("{e}"))?;
    if (0.0..=1.0).contains(&threshold) {
        Ok(threshold)
    } else {
        Err(format!("{threshold} is not between 0.0 and 1.0"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Args {
        Args::parse_from(args)
    }

    #[test]
    fn defaults() {
        let args = parse(&["dublette", "/tmp"]);
        assert_eq!(args.directories, vec![PathBuf::from("/tmp")]);
        assert_eq!(args.threshold, 1);
        assert!(!args.dry_run);
        assert!(!args.delete_empty);
        assert!(!args.yes);
        assert!(!args.quiet);
        assert!(!args.verbose);
        assert!(!args.no_color);
        assert!(!args.json);
        assert!(args.only.is_none());
        assert!(args.keep_in.is_empty());
    }

    #[test]
    fn keep_in_repeats_preserve_order() {
        let args = parse(&["dublette", "/tmp", "--keep-in", "/a", "--keep-in", "/b"]);
        assert_eq!(args.keep_in, vec![PathBuf::from("/a"), PathBuf::from("/b")]);
    }

    #[test]
    fn multiple_directories() {
        let args = parse(&["dublette", "/tmp/a", "/tmp/b", "/tmp/c"]);
        assert_eq!(
            args.directories,
            vec![
                PathBuf::from("/tmp/a"),
                PathBuf::from("/tmp/b"),
                PathBuf::from("/tmp/c")
            ]
        );
    }

    #[test]
    fn all_flags() {
        let args = parse(&[
            "dublette",
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
        let args = parse(&["dublette", "/tmp", "--only", "videos"]);
        assert!(matches!(args.only, Some(MediaFilter::Videos)));
    }

    #[test]
    fn only_audio() {
        let args = parse(&["dublette", "/tmp", "--only", "audio"]);
        assert!(matches!(args.only, Some(MediaFilter::Audio)));
    }

    #[test]
    fn audio_match_defaults_to_recording() {
        let args = parse(&["dublette", "/tmp"]);
        assert_eq!(args.audio_match, MatchStrategy::Recording);
        assert!(args.audio_threshold.is_none());
    }

    #[test]
    fn audio_match_encoding() {
        let args = parse(&["dublette", "/tmp", "--audio-match", "encoding"]);
        assert_eq!(args.audio_match, MatchStrategy::Encoding);
    }

    #[test]
    fn audio_threshold_parses() {
        let args = parse(&["dublette", "/tmp", "--audio-threshold", "0.3"]);
        assert_eq!(args.audio_threshold, Some(0.3));
    }

    #[test]
    fn audio_threshold_rejects_out_of_range() {
        assert!(Args::try_parse_from(["dublette", "/tmp", "--audio-threshold", "1.5"]).is_err());
        assert!(Args::try_parse_from(["dublette", "/tmp", "--audio-threshold", "-0.1"]).is_err());
    }

    #[test]
    fn missing_directory_fails() {
        let result = Args::try_parse_from(&["dublette"]);
        assert!(result.is_err());
    }

    #[test]
    fn version_long_flag() {
        let err = Args::try_parse_from(["dublette", "--version"])
            .map(|_| ())
            .unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
        assert!(err.to_string().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn version_short_flag() {
        let err = Args::try_parse_from(["dublette", "-V"])
            .map(|_| ())
            .unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }
}
