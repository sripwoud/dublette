use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;

use img_hash::ImageHash;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;

use crate::{delete, hash, scan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaKind {
    Image,
    Video,
}

pub struct Config {
    pub threshold: u32,
    pub only: Option<MediaKind>,
    pub include_empty: bool,
}

pub struct SkippedFile {
    pub path: PathBuf,
    pub reason: String,
}

pub struct HashedFile {
    pub path: PathBuf,
    pub hash: ImageHash,
}

pub struct DuplicateGroup {
    pub kind: MediaKind,
    pub keep: PathBuf,
    pub duplicates: Vec<PathBuf>,
}

pub struct DeduplicationReport {
    pub groups: Vec<DuplicateGroup>,
    pub empty_files: Vec<PathBuf>,
    pub skipped: Vec<SkippedFile>,
    pub to_delete: Vec<PathBuf>,
}

pub trait Progress: Sync {
    fn phase_start(&self, label: &str, total: u64);
    fn tick(&self);
    fn phase_finish(&self);
    fn diag(&self, msg: &str);
}

pub struct NoopProgress;

impl Progress for NoopProgress {
    fn phase_start(&self, _label: &str, _total: u64) {}
    fn tick(&self) {}
    fn phase_finish(&self) {}
    fn diag(&self, _msg: &str) {}
}

pub struct IndicatifProgress {
    current: Mutex<Option<ProgressBar>>,
    verbose: bool,
}

impl IndicatifProgress {
    pub fn new(verbose: bool) -> Self {
        Self {
            current: Mutex::new(None),
            verbose,
        }
    }
}

impl Progress for IndicatifProgress {
    fn phase_start(&self, label: &str, total: u64) {
        let mut current = self.current.lock().expect("progress mutex poisoned");
        if let Some(prev) = current.take() {
            prev.finish_and_clear();
        }
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40}] {pos}/{len} ({eta})")
                .expect("valid template")
                .progress_chars("=> "),
        );
        pb.set_message(label.to_string());
        *current = Some(pb);
    }

    fn tick(&self) {
        if let Some(pb) = self
            .current
            .lock()
            .expect("progress mutex poisoned")
            .as_ref()
        {
            pb.inc(1);
        }
    }

    fn phase_finish(&self) {
        if let Some(pb) = self.current.lock().expect("progress mutex poisoned").take() {
            pb.finish_and_clear();
        }
    }

    fn diag(&self, msg: &str) {
        if self.verbose {
            eprintln!("  {msg}");
        }
    }
}

pub fn plan(
    dirs: &[PathBuf],
    config: &Config,
    progress: &dyn Progress,
) -> eyre::Result<DeduplicationReport> {
    let mut skipped: Vec<SkippedFile> = Vec::new();
    let mut groups: Vec<DuplicateGroup> = Vec::new();

    let empty_files = if config.include_empty {
        delete::find_empty_files(dirs)?
    } else {
        Vec::new()
    };

    if !matches!(config.only, Some(MediaKind::Video)) {
        let exts: HashSet<&str> = scan::IMAGE_EXTENSIONS.iter().copied().collect();
        let files = scan::collect_files(dirs, &exts)?;
        let (hashed, image_skipped) = hash_in_parallel(&files, progress, "Hashing images", |p| {
            hash::compute_image_hash(p)
        });
        skipped.extend(image_skipped);
        let adjacency = scan::pairwise_compare(&hashed, config.threshold);
        groups.extend(scan::build_duplicate_groups(&adjacency, MediaKind::Image));
    }

    if !matches!(config.only, Some(MediaKind::Image))
        && let Ok(ffmpeg) = hash::find_ffmpeg()
    {
        let exts: HashSet<&str> = scan::VIDEO_EXTENSIONS.iter().copied().collect();
        let files = scan::collect_files(dirs, &exts)?;
        let (hashed, video_skipped) = hash_in_parallel(&files, progress, "Hashing videos", |p| {
            hash::extract_video_frame_hash(p, &ffmpeg)
        });
        skipped.extend(video_skipped);
        let adjacency = scan::pairwise_compare(&hashed, config.threshold);
        groups.extend(scan::build_duplicate_groups(&adjacency, MediaKind::Video));
    }

    let mut to_delete: Vec<PathBuf> = groups
        .iter()
        .flat_map(|g| g.duplicates.iter().cloned())
        .collect();
    if config.include_empty {
        to_delete.extend(empty_files.iter().cloned());
    }

    Ok(DeduplicationReport {
        groups,
        empty_files,
        skipped,
        to_delete,
    })
}

fn hash_in_parallel<F>(
    files: &[PathBuf],
    progress: &dyn Progress,
    label: &str,
    hash_fn: F,
) -> (Vec<HashedFile>, Vec<SkippedFile>)
where
    F: Fn(&PathBuf) -> eyre::Result<ImageHash> + Sync,
{
    progress.phase_start(label, files.len() as u64);
    let results: Vec<Result<HashedFile, SkippedFile>> = files
        .par_iter()
        .map(|f| {
            let outcome = hash_fn(f);
            progress.tick();
            match outcome {
                Ok(h) => {
                    progress.diag(&format!("{} -> {:?}", f.display(), h));
                    Ok(HashedFile {
                        path: f.clone(),
                        hash: h,
                    })
                }
                Err(e) => Err(SkippedFile {
                    path: f.clone(),
                    reason: format!("{e}"),
                }),
            }
        })
        .collect();
    progress.phase_finish();

    let mut hashed: Vec<HashedFile> = Vec::new();
    let mut skipped: Vec<SkippedFile> = Vec::new();
    for r in results {
        match r {
            Ok(h) => hashed.push(h),
            Err(s) => skipped.push(s),
        }
    }
    (hashed, skipped)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[test]
    fn noop_progress_does_nothing() {
        let p = NoopProgress;
        p.phase_start("test", 10);
        p.tick();
        p.tick();
        p.diag("hello");
        p.phase_finish();
    }

    struct RecordingProgress {
        events: Mutex<Vec<String>>,
    }

    impl RecordingProgress {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    impl Progress for RecordingProgress {
        fn phase_start(&self, label: &str, total: u64) {
            self.events
                .lock()
                .unwrap()
                .push(format!("start:{label}:{total}"));
        }

        fn tick(&self) {
            self.events.lock().unwrap().push("tick".to_string());
        }

        fn phase_finish(&self) {
            self.events.lock().unwrap().push("finish".to_string());
        }

        fn diag(&self, msg: &str) {
            self.events.lock().unwrap().push(format!("diag:{msg}"));
        }
    }

    fn write_gradient(path: &std::path::Path) {
        let img: image::RgbImage =
            image::ImageBuffer::from_fn(64, 64, |x, _| image::Rgb([x as u8, x as u8, x as u8]));
        img.save(path).unwrap();
    }

    fn write_checkerboard(path: &std::path::Path, block: u32) {
        let img: image::RgbImage = image::ImageBuffer::from_fn(64, 64, |x, y| {
            if ((x / block) + (y / block)).is_multiple_of(2) {
                image::Rgb([255, 255, 255])
            } else {
                image::Rgb([0, 0, 0])
            }
        });
        img.save(path).unwrap();
    }

    fn default_config() -> Config {
        Config {
            threshold: 0,
            only: None,
            include_empty: false,
        }
    }

    fn dirs(d: &tempfile::TempDir) -> Vec<PathBuf> {
        vec![d.path().to_path_buf()]
    }

    #[test]
    fn recording_progress_captures_phase_lifecycle() {
        let p = RecordingProgress::new();
        p.phase_start("hashing", 3);
        p.tick();
        p.tick();
        p.diag("a -> 0xff");
        p.tick();
        p.phase_finish();

        assert_eq!(
            p.events(),
            vec![
                "start:hashing:3",
                "tick",
                "tick",
                "diag:a -> 0xff",
                "tick",
                "finish",
            ]
        );
    }

    #[test]
    fn plan_empty_dirs_returns_empty_report() {
        let dir = tempfile::tempdir().unwrap();
        let report = plan(&dirs(&dir), &default_config(), &NoopProgress).unwrap();

        assert!(report.groups.is_empty());
        assert!(report.empty_files.is_empty());
        assert!(report.skipped.is_empty());
        assert!(report.to_delete.is_empty());
    }

    #[test]
    fn plan_identical_images_yield_one_group_kind_image() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        write_gradient(&a);
        write_gradient(&b);

        let report = plan(&dirs(&dir), &default_config(), &NoopProgress).unwrap();

        assert_eq!(report.groups.len(), 1);
        assert_eq!(report.groups[0].kind, MediaKind::Image);
        assert_eq!(report.groups[0].keep, a);
        assert_eq!(report.groups[0].duplicates, vec![b]);
    }

    #[test]
    fn plan_threshold_zero_excludes_near_matches() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        write_checkerboard(&a, 8);
        write_checkerboard(&b, 16);

        let h_a = hash::compute_image_hash(&a).unwrap();
        let h_b = hash::compute_image_hash(&b).unwrap();
        let distance = h_a.dist(&h_b);
        assert!(distance > 0, "test setup expects non-zero distance");

        let report = plan(&dirs(&dir), &default_config(), &NoopProgress).unwrap();

        assert!(
            report.groups.is_empty(),
            "threshold=0 should exclude near match (distance={distance})"
        );
    }

    #[test]
    fn plan_threshold_loose_includes_near_matches() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        write_checkerboard(&a, 8);
        write_checkerboard(&b, 16);

        let h_a = hash::compute_image_hash(&a).unwrap();
        let h_b = hash::compute_image_hash(&b).unwrap();
        let distance = h_a.dist(&h_b);

        let config = Config {
            threshold: distance,
            only: None,
            include_empty: false,
        };
        let report = plan(&dirs(&dir), &config, &NoopProgress).unwrap();

        assert_eq!(report.groups.len(), 1);
        assert_eq!(report.groups[0].kind, MediaKind::Image);
    }

    #[test]
    fn plan_only_images_skips_videos() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        write_gradient(&a);
        write_gradient(&b);
        std::fs::write(dir.path().join("garbage.mp4"), b"not-a-video").unwrap();

        let config = Config {
            threshold: 0,
            only: Some(MediaKind::Image),
            include_empty: false,
        };
        let report = plan(&dirs(&dir), &config, &NoopProgress).unwrap();

        assert_eq!(report.groups.len(), 1);
        assert_eq!(report.groups[0].kind, MediaKind::Image);
        assert!(
            report
                .skipped
                .iter()
                .all(|s| s.path.extension().and_then(|e| e.to_str()) != Some("mp4")),
            "video file should not have been processed"
        );
    }

    #[test]
    fn plan_only_videos_skips_images() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        write_gradient(&a);
        write_gradient(&b);

        let config = Config {
            threshold: 0,
            only: Some(MediaKind::Video),
            include_empty: false,
        };
        let report = plan(&dirs(&dir), &config, &NoopProgress).unwrap();

        assert!(
            report.groups.iter().all(|g| g.kind != MediaKind::Image),
            "image groups should be absent"
        );
        assert!(
            report
                .skipped
                .iter()
                .all(|s| s.path.extension().and_then(|e| e.to_str()) != Some("png")),
            "image files should not have been processed"
        );
    }

    #[test]
    fn plan_include_empty_populates_empty_files_and_to_delete() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.jpg");
        std::fs::write(&empty, []).unwrap();

        let config = Config {
            threshold: 0,
            only: None,
            include_empty: true,
        };
        let report = plan(&dirs(&dir), &config, &NoopProgress).unwrap();

        assert_eq!(report.empty_files, vec![empty.clone()]);
        assert!(report.to_delete.contains(&empty));
    }

    #[test]
    fn plan_exclude_empty_omits_empty_files() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.jpg");
        std::fs::write(&empty, []).unwrap();

        let report = plan(&dirs(&dir), &default_config(), &NoopProgress).unwrap();

        assert!(report.empty_files.is_empty());
        assert!(!report.to_delete.contains(&empty));
    }

    #[test]
    fn plan_unreadable_image_recorded_as_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.jpg");
        std::fs::write(&bad, b"not-an-image-payload").unwrap();

        let report = plan(&dirs(&dir), &default_config(), &NoopProgress).unwrap();

        assert!(report.groups.is_empty());
        let skipped: Vec<&SkippedFile> = report.skipped.iter().filter(|s| s.path == bad).collect();
        assert_eq!(
            skipped.len(),
            1,
            "expected exactly one skipped entry for bad.jpg"
        );
        assert!(
            !skipped[0].reason.is_empty(),
            "skipped reason should describe the failure"
        );
    }

    #[test]
    fn plan_to_delete_uses_pathbuf_not_string() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        write_gradient(&a);
        write_gradient(&b);

        let report = plan(&dirs(&dir), &default_config(), &NoopProgress).unwrap();

        let _: &Vec<PathBuf> = &report.to_delete;
        assert_eq!(report.to_delete, vec![b]);
    }

    #[test]
    fn plan_multiple_directories_merge_into_one_pipeline() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let a = dir1.path().join("a.png");
        let b = dir2.path().join("b.png");
        write_gradient(&a);
        write_gradient(&b);

        let directories = vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()];
        let report = plan(&directories, &default_config(), &NoopProgress).unwrap();

        assert_eq!(report.groups.len(), 1);
        let group = &report.groups[0];
        let mut all = vec![group.keep.clone()];
        all.extend(group.duplicates.iter().cloned());
        all.sort();
        let mut expected = vec![a, b];
        expected.sort();
        assert_eq!(all, expected);
    }
}
