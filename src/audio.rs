use std::fmt;
use std::fs::OpenOptions;
use std::hash::{DefaultHasher, Hasher};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use clap::ValueEnum;
use lofty::config::WriteOptions;
use lofty::prelude::*;
use rusty_chromaprint::{Configuration, Fingerprinter, match_fingerprints};

use crate::skip::{SkipError, SkipReason};

pub const DEFAULT_THRESHOLD: f64 = 0.1;

const PCM_SAMPLE_RATE: u32 = 11025;
const MAX_DECODE_SECONDS: u32 = 120;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum MatchStrategy {
    Recording,
    Encoding,
}

pub struct Fingerprint(Vec<u32>);

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fingerprint({} items)", self.0.len())
    }
}

fn chromaprint_config() -> &'static Configuration {
    static CONFIG: OnceLock<Configuration> = OnceLock::new();
    CONFIG.get_or_init(Configuration::preset_test2)
}

pub fn fingerprint(path: &Path, ffmpeg: &Path) -> Result<Fingerprint, SkipError> {
    let mut child = Command::new(ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(path)
        .args([
            "-map",
            "0:a:0",
            "-f",
            "s16le",
            "-acodec",
            "pcm_s16le",
            "-ar",
            &PCM_SAMPLE_RATE.to_string(),
            "-ac",
            "1",
            "-t",
            &MAX_DECODE_SECONDS.to_string(),
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut pcm = Vec::new();
    child
        .stdout
        .take()
        .expect("stdout was piped")
        .read_to_end(&mut pcm)?;
    let status = child.wait()?;
    if !status.success() {
        return Err(SkipError::new(
            SkipReason::DecodeFailed,
            format!("ffmpeg could not decode audio from {}", path.display()),
        ));
    }

    let samples: Vec<i16> = pcm
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();

    let mut printer = Fingerprinter::new(chromaprint_config());
    printer.start(PCM_SAMPLE_RATE, 1).map_err(|e| {
        SkipError::new(
            SkipReason::DecodeFailed,
            format!("fingerprinter rejected PCM stream: {e}"),
        )
    })?;
    printer.consume(&samples);
    printer.finish();

    let items = printer.fingerprint().to_vec();
    if items.is_empty() {
        return Err(SkipError::new(
            SkipReason::TooShort,
            format!("audio in {} is too short to fingerprint", path.display()),
        ));
    }
    Ok(Fingerprint(items))
}

pub fn dissimilarity(a: &Fingerprint, b: &Fingerprint) -> f64 {
    let total = a.0.len().max(b.0.len());
    if total == 0 {
        return 1.0;
    }
    let Ok(segments) = match_fingerprints(&a.0, &b.0, chromaprint_config()) else {
        return 1.0;
    };

    let matched_error: f64 = segments
        .iter()
        .map(|s| s.items_count as f64 * (s.score / 32.0))
        .sum();
    let matched_items: usize = segments.iter().map(|s| s.items_count).sum();
    let unmatched = total.saturating_sub(matched_items) as f64;
    (matched_error + unmatched) / total as f64
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AudioQuality {
    pub lossless: bool,
    pub bitrate: u32,
}

pub fn quality(path: &Path) -> AudioQuality {
    let Ok(tagged) = lofty::read_from_path(path) else {
        return AudioQuality {
            lossless: false,
            bitrate: 0,
        };
    };
    AudioQuality {
        lossless: matches!(
            tagged.file_type(),
            lofty::file::FileType::Flac | lofty::file::FileType::Wav | lofty::file::FileType::Aiff
        ),
        bitrate: tagged.properties().audio_bitrate().unwrap_or(0),
    }
}

pub fn quality_keep(mut members: Vec<PathBuf>) -> (PathBuf, Vec<PathBuf>) {
    members.sort();
    let qualities: Vec<AudioQuality> = members.iter().map(|p| quality(p)).collect();
    let mut best = 0;
    for (i, q) in qualities.iter().enumerate() {
        if *q > qualities[best] {
            best = i;
        }
    }
    let keep = members.remove(best);
    (keep, members)
}

pub fn encoding_hash(path: &Path) -> Result<u64, SkipError> {
    let workdir = tempfile::tempdir()?;
    let stripped = workdir.path().join("stripped");
    std::fs::copy(path, &stripped)?;

    let mut file = OpenOptions::new().read(true).write(true).open(&stripped)?;
    let tagged = lofty::read_from(&mut file).map_err(|e| {
        SkipError::new(
            SkipReason::UnsupportedContainer,
            format!("failed to parse audio container {}: {e}", path.display()),
        )
    })?;
    for tag in tagged.tags() {
        file.rewind()?;
        tag.remove_from(&mut file, WriteOptions::default())
            .map_err(|e| {
                SkipError::new(
                    SkipReason::Unreadable,
                    format!("failed to exclude tag region in {}: {e}", path.display()),
                )
            })?;
    }

    file.rewind()?;
    let mut hasher = DefaultHasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.write(&buffer[..read]);
    }
    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use lofty::tag::{Tag, TagType};

    use super::*;

    fn synthetic_fingerprint(range: std::ops::Range<u32>) -> Fingerprint {
        Fingerprint(range.map(|i| i << 20).collect())
    }

    #[test]
    fn identical_fingerprints_have_zero_dissimilarity() {
        let a = synthetic_fingerprint(0..100);
        let b = synthetic_fingerprint(0..100);
        assert_eq!(dissimilarity(&a, &b), 0.0);
    }

    #[test]
    fn disjoint_fingerprints_have_full_dissimilarity() {
        let a = synthetic_fingerprint(0..100);
        let b = synthetic_fingerprint(200..300);
        assert_eq!(dissimilarity(&a, &b), 1.0);
    }

    #[test]
    fn empty_fingerprints_are_fully_dissimilar() {
        let empty = Fingerprint(Vec::new());
        let full = synthetic_fingerprint(0..100);
        assert_eq!(dissimilarity(&empty, &full), 1.0);
        assert_eq!(dissimilarity(&empty, &empty), 1.0);
    }

    fn find_ffmpeg() -> Option<PathBuf> {
        which::which("ffmpeg").ok()
    }

    const MELODY_A: &str = "sin(2*PI*(220+55*floor(t*2))*t)";
    const MELODY_B: &str = "sin(2*PI*(392+98*floor(t*3))*t)";

    fn synth_recording(ffmpeg: &Path, out: &Path, melody: &str) {
        let status = Command::new(ffmpeg)
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("aevalsrc={melody}:s=44100:d=10"),
            ])
            .arg(out)
            .status()
            .unwrap();
        assert!(status.success(), "ffmpeg failed to synthesize {out:?}");
    }

    fn encode(ffmpeg: &Path, input: &Path, out: &Path) {
        let status = Command::new(ffmpeg)
            .args(["-v", "error", "-y", "-i"])
            .arg(input)
            .arg(out)
            .status()
            .unwrap();
        assert!(status.success(), "ffmpeg failed to encode {out:?}");
    }

    #[test]
    fn same_recording_across_codecs_is_within_default_threshold() {
        let Some(ffmpeg) = find_ffmpeg() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let master = dir.path().join("master.wav");
        let flac = dir.path().join("a.flac");
        let mp3 = dir.path().join("b.mp3");
        synth_recording(&ffmpeg, &master, MELODY_A);
        encode(&ffmpeg, &master, &flac);
        encode(&ffmpeg, &master, &mp3);

        let d = dissimilarity(
            &fingerprint(&flac, &ffmpeg).unwrap(),
            &fingerprint(&mp3, &ffmpeg).unwrap(),
        );
        assert!(
            d <= DEFAULT_THRESHOLD,
            "same recording should be within default threshold (dissimilarity={d})"
        );
    }

    #[test]
    fn different_recordings_exceed_default_threshold() {
        let Some(ffmpeg) = find_ffmpeg() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.flac");
        let b = dir.path().join("b.flac");
        synth_recording(&ffmpeg, &a, MELODY_A);
        synth_recording(&ffmpeg, &b, MELODY_B);

        let d = dissimilarity(
            &fingerprint(&a, &ffmpeg).unwrap(),
            &fingerprint(&b, &ffmpeg).unwrap(),
        );
        assert!(
            d > DEFAULT_THRESHOLD,
            "different recordings should exceed default threshold (dissimilarity={d})"
        );
    }

    #[test]
    fn lossless_outranks_any_lossy_bitrate() {
        let flac = AudioQuality {
            lossless: true,
            bitrate: 400,
        };
        let mp3 = AudioQuality {
            lossless: false,
            bitrate: 320,
        };
        assert!(flac > mp3);
    }

    #[test]
    fn higher_bitrate_wins_within_same_fidelity_class() {
        let high = AudioQuality {
            lossless: false,
            bitrate: 320,
        };
        let low = AudioQuality {
            lossless: false,
            bitrate: 128,
        };
        assert!(high > low);
    }

    #[test]
    fn quality_of_wav_is_lossless() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("a.wav");
        write_wav(&wav, &tone(440.0, 4410));

        assert!(quality(&wav).lossless);
    }

    #[test]
    fn quality_of_unparseable_file_is_bottom_rank() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.mp3");
        fs::write(&bad, b"not-audio-at-all").unwrap();

        assert_eq!(
            quality(&bad),
            AudioQuality {
                lossless: false,
                bitrate: 0
            }
        );
    }

    #[test]
    fn quality_keep_prefers_lossless_over_alphabetical_order() {
        let Some(ffmpeg) = find_ffmpeg() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let mp3 = dir.path().join("a.mp3");
        let flac = dir.path().join("z.flac");
        synth_recording(&ffmpeg, &mp3, MELODY_A);
        synth_recording(&ffmpeg, &flac, MELODY_A);

        let (keep, duplicates) = quality_keep(vec![mp3.clone(), flac.clone()]);
        assert_eq!(keep, flac);
        assert_eq!(duplicates, vec![mp3]);
    }

    #[test]
    fn quality_keep_falls_back_to_alphabetical_on_equal_quality() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let z = dir.path().join("z.wav");
        write_wav(&a, &tone(440.0, 4410));
        fs::copy(&a, &z).unwrap();

        let (keep, duplicates) = quality_keep(vec![z.clone(), a.clone()]);
        assert_eq!(keep, a);
        assert_eq!(duplicates, vec![z]);
    }

    #[test]
    fn fingerprint_of_unreadable_file_errors() {
        let Some(ffmpeg) = find_ffmpeg() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.mp3");
        fs::write(&bad, b"not-audio-at-all").unwrap();

        assert!(fingerprint(&bad, &ffmpeg).is_err());
    }

    fn write_wav(path: &Path, samples: &[i16]) {
        let data_len = (samples.len() * 2) as u32;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&44100u32.to_le_bytes());
        bytes.extend_from_slice(&(44100u32 * 2).to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        fs::write(path, bytes).unwrap();
    }

    fn tone(frequency: f64, len: usize) -> Vec<i16> {
        (0..len)
            .map(|i| {
                let t = i as f64 / 44100.0;
                ((t * frequency * 2.0 * std::f64::consts::PI).sin() * 20000.0) as i16
            })
            .collect()
    }

    fn retag(path: &Path, title: &str) {
        let mut tag = Tag::new(TagType::Id3v2);
        tag.set_title(title.to_string());
        tag.save_to_path(path, WriteOptions::default()).unwrap();
    }

    #[test]
    fn retagged_copy_has_same_encoding_hash() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("a.wav");
        let retagged = dir.path().join("b.wav");
        write_wav(&original, &tone(440.0, 4410));
        fs::copy(&original, &retagged).unwrap();
        retag(&retagged, "renamed");

        assert_ne!(
            fs::read(&original).unwrap(),
            fs::read(&retagged).unwrap(),
            "test setup expects tag to change file bytes"
        );
        assert_eq!(
            encoding_hash(&original).unwrap(),
            encoding_hash(&retagged).unwrap()
        );
    }

    #[test]
    fn different_audio_has_different_encoding_hash() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        write_wav(&a, &tone(440.0, 4410));
        write_wav(&b, &tone(880.0, 4410));

        assert_ne!(encoding_hash(&a).unwrap(), encoding_hash(&b).unwrap());
    }

    #[test]
    fn re_encode_of_same_recording_has_different_encoding_hash() {
        let Some(ffmpeg) = find_ffmpeg() else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let master = dir.path().join("master.wav");
        let flac = dir.path().join("a.flac");
        let wav = dir.path().join("b.wav");
        synth_recording(&ffmpeg, &master, MELODY_A);
        encode(&ffmpeg, &master, &flac);
        encode(&ffmpeg, &master, &wav);

        assert_ne!(
            encoding_hash(&flac).unwrap(),
            encoding_hash(&wav).unwrap(),
            "a lossless transcode is a recording match, never an encoding match"
        );
    }

    #[test]
    fn unparseable_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.mp3");
        fs::write(&bad, b"not-audio-at-all").unwrap();

        assert!(encoding_hash(&bad).is_err());
    }

    #[test]
    fn nonexistent_file_errors() {
        assert!(encoding_hash(Path::new("/nonexistent.mp3")).is_err());
    }
}
