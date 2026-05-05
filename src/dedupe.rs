use std::path::PathBuf;
use std::sync::Mutex;

use indicatif::{ProgressBar, ProgressStyle};

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
}
