use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkipReason {
    DecodeFailed,
    TooShort,
    UnsupportedContainer,
    Unreadable,
}

impl SkipReason {
    pub fn tag(self) -> &'static str {
        match self {
            SkipReason::DecodeFailed => "decode_failed",
            SkipReason::TooShort => "too_short",
            SkipReason::UnsupportedContainer => "unsupported_container",
            SkipReason::Unreadable => "unreadable",
        }
    }
}

#[derive(Debug)]
pub struct SkipError {
    pub reason: SkipReason,
    pub detail: String,
}

impl SkipError {
    pub fn new(reason: SkipReason, detail: impl Into<String>) -> Self {
        Self {
            reason,
            detail: detail.into(),
        }
    }
}

impl From<std::io::Error> for SkipError {
    fn from(error: std::io::Error) -> Self {
        Self::new(SkipReason::Unreadable, error.to_string())
    }
}

pub struct SkippedFile {
    pub path: PathBuf,
    pub reason: SkipReason,
    pub detail: String,
}

impl SkippedFile {
    pub fn new(path: PathBuf, error: SkipError) -> Self {
        Self {
            path,
            reason: error.reason,
            detail: error.detail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_stable_snake_case() {
        assert_eq!(SkipReason::DecodeFailed.tag(), "decode_failed");
        assert_eq!(SkipReason::TooShort.tag(), "too_short");
        assert_eq!(
            SkipReason::UnsupportedContainer.tag(),
            "unsupported_container"
        );
        assert_eq!(SkipReason::Unreadable.tag(), "unreadable");
    }

    #[test]
    fn io_errors_are_unreadable() {
        let error = SkipError::from(std::io::Error::other("disk on fire"));
        assert_eq!(error.reason, SkipReason::Unreadable);
        assert_eq!(error.detail, "disk on fire");
    }
}
