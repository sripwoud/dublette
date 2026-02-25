pub mod cli;
pub mod delete;
pub mod hash;
pub mod report;
pub mod scan;

use std::collections::HashSet;
use std::path::Path;

use indicatif::{ProgressBar, ProgressStyle};

use cli::{Args, MediaFilter};
use scan::{HashedFile, IMAGE_EXTENSIONS, VIDEO_EXTENSIONS};

fn make_progress_bar(len: u64, msg: &str, quiet: bool) -> ProgressBar {
    if quiet {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40}] {pos}/{len} ({eta})")
            .expect("valid template")
            .progress_chars("=> "),
    );
    pb.set_message(msg.to_string());
    pb
}

fn hash_images(files: &[std::path::PathBuf], directory: &Path, args: &Args) -> Vec<HashedFile> {
    let pb = make_progress_bar(files.len() as u64, "Hashing images", args.quiet);
    let mut hashes = Vec::new();

    for f in files {
        match hash::compute_image_hash(f) {
            Ok(h) => {
                let rel = f.strip_prefix(directory).unwrap_or(f);
                let key = rel.to_string_lossy().to_string();
                if args.verbose {
                    eprintln!("  {} -> {:?}", key, h);
                }
                hashes.push(HashedFile {
                    relative_path: key,
                    hash: h,
                });
            }
            Err(e) => eprintln!("  Warning: skipping {}: {e}", f.display()),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();
    hashes
}

fn hash_videos(
    files: &[std::path::PathBuf],
    directory: &Path,
    ffmpeg: &Path,
    args: &Args,
) -> Vec<HashedFile> {
    let pb = make_progress_bar(files.len() as u64, "Hashing videos", args.quiet);
    let mut hashes = Vec::new();

    for f in files {
        match hash::extract_video_frame_hash(f, ffmpeg) {
            Ok(h) => {
                let rel = f.strip_prefix(directory).unwrap_or(f);
                let key = rel.to_string_lossy().to_string();
                if args.verbose {
                    eprintln!("  {} -> {:?}", key, h);
                }
                hashes.push(HashedFile {
                    relative_path: key,
                    hash: h,
                });
            }
            Err(e) => eprintln!("  Warning: skipping {}: {e}", f.display()),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();
    hashes
}

fn compare_hashes(
    hashes: &[HashedFile],
    threshold: u32,
    label: &str,
    args: &Args,
) -> Vec<scan::DuplicateGroup> {
    let total_pairs = (hashes.len() * hashes.len().saturating_sub(1)) / 2;
    let pb = make_progress_bar(
        total_pairs as u64,
        &format!("Comparing {label}"),
        args.quiet,
    );

    let mut duplicates = std::collections::HashMap::new();
    for h in hashes {
        duplicates
            .entry(h.relative_path.clone())
            .or_insert_with(Vec::new);
    }

    for i in 0..hashes.len() {
        for j in (i + 1)..hashes.len() {
            let distance = hashes[i].hash.dist(&hashes[j].hash);
            if args.verbose {
                eprintln!(
                    "  {} <-> {}: distance={}",
                    hashes[i].relative_path, hashes[j].relative_path, distance
                );
            }
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
            pb.inc(1);
        }
    }

    pb.finish_and_clear();
    scan::build_duplicate_groups(&duplicates)
}

fn process_media(
    directory: &Path,
    extensions: &HashSet<&str>,
    label: &str,
    hash_fn: impl Fn(&[std::path::PathBuf], &Path, &Args) -> Vec<HashedFile>,
    args: &Args,
    all_groups: &mut Vec<scan::DuplicateGroup>,
) -> eyre::Result<()> {
    let files = scan::collect_files(directory, extensions)?;
    if files.is_empty() {
        if !args.json {
            println!("No {label}s found.");
        }
        return Ok(());
    }

    if !args.quiet && !args.json {
        eprintln!("Scanning {} {label}(s)...", files.len());
    }

    let hashes = hash_fn(&files, directory, args);
    let groups = compare_hashes(&hashes, args.threshold, label, args);

    if !args.json {
        if groups.is_empty() {
            println!("No duplicate {label}s found.");
        } else {
            println!("{}", report::format_table(&groups, args.dry_run, label));
        }
    }

    all_groups.extend(groups);
    Ok(())
}

pub fn run(args: &Args) -> eyre::Result<bool> {
    let directory = &args.directory;
    let mut total_deleted = 0usize;
    let mut all_groups: Vec<scan::DuplicateGroup> = Vec::new();
    let mut empty_files_rel: Vec<String> = Vec::new();

    if args.delete_empty {
        let empty = delete::find_empty_files(directory)?;
        if !empty.is_empty() {
            empty_files_rel = empty
                .iter()
                .map(|p| {
                    p.strip_prefix(directory)
                        .unwrap_or(p)
                        .to_string_lossy()
                        .to_string()
                })
                .collect();

            if !args.json {
                println!(
                    "{}",
                    report::format_empty_table(&empty_files_rel, args.dry_run)
                );
            }

            if !args.dry_run {
                total_deleted += delete::delete_files(&empty, directory, "empty", args.yes)?;
            }
        }
    }

    if !matches!(args.only, Some(MediaFilter::Videos)) {
        let image_exts: HashSet<&str> = IMAGE_EXTENSIONS.iter().copied().collect();
        process_media(
            directory,
            &image_exts,
            "image",
            hash_images,
            args,
            &mut all_groups,
        )?;
    }

    if !matches!(args.only, Some(MediaFilter::Images)) {
        match hash::find_ffmpeg() {
            Ok(ffmpeg) => {
                let video_exts: HashSet<&str> = VIDEO_EXTENSIONS.iter().copied().collect();
                process_media(
                    directory,
                    &video_exts,
                    "video",
                    |files, dir, a| hash_videos(files, dir, &ffmpeg, a),
                    args,
                    &mut all_groups,
                )?;
            }
            Err(_) => {
                if !args.quiet && !args.json {
                    eprintln!("Warning: ffmpeg not found, skipping video processing");
                }
            }
        }
    }

    let found_duplicates = !all_groups.is_empty();

    if args.json {
        println!(
            "{}",
            report::format_json(&all_groups, &empty_files_rel, args.dry_run)
        );
    }

    if !args.dry_run && found_duplicates {
        let to_delete = report::resolve_deletions(&all_groups, directory);
        total_deleted += delete::delete_files(&to_delete, directory, "duplicate", args.yes)?;
    }

    if !args.json {
        if args.dry_run && found_duplicates {
            let total: usize = all_groups.iter().map(|g| g.duplicates.len()).sum();
            println!("\n[dry run] {} file(s) would be deleted.", total);
        } else if total_deleted > 0 {
            eprintln!("\nRemoved {total_deleted} duplicate(s) total.");
        }
    }

    Ok(found_duplicates)
}
