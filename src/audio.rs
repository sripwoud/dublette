use std::fs::OpenOptions;
use std::hash::{DefaultHasher, Hasher};
use std::io::{Read, Seek};
use std::path::Path;

use lofty::config::WriteOptions;
use lofty::prelude::*;

pub fn encoding_hash(path: &Path) -> eyre::Result<u64> {
    let workdir = tempfile::tempdir()?;
    let stripped = workdir.path().join("stripped");
    std::fs::copy(path, &stripped)?;

    let mut file = OpenOptions::new().read(true).write(true).open(&stripped)?;
    let tagged = lofty::read_from(&mut file)
        .map_err(|e| eyre::eyre!("failed to parse audio container {}: {e}", path.display()))?;
    for tag in tagged.tags() {
        file.rewind()?;
        tag.remove_from(&mut file, WriteOptions::default())
            .map_err(|e| eyre::eyre!("failed to exclude tag region in {}: {e}", path.display()))?;
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
    use std::path::Path;

    use lofty::tag::{Tag, TagType};

    use super::*;

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
