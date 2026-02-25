use std::path::{Path, PathBuf};
use std::process::Command;

use img_hash::{HashAlg, HasherConfig, ImageHash};

fn hasher() -> img_hash::Hasher {
    HasherConfig::new()
        .hash_alg(HashAlg::DoubleGradient)
        .hash_size(8, 8)
        .to_hasher()
}

pub fn compute_image_hash(path: &Path) -> eyre::Result<ImageHash> {
    let img =
        image::open(path).map_err(|e| eyre::eyre!("failed to open {}: {e}", path.display()))?;
    Ok(hasher().hash_image(&img))
}

pub fn find_ffmpeg() -> eyre::Result<PathBuf> {
    which::which("ffmpeg").map_err(|_| eyre::eyre!("ffmpeg not found on PATH"))
}

fn run_ffmpeg_extract(ffmpeg: &Path, video: &Path, seek: &str, output: &Path) -> bool {
    Command::new(ffmpeg)
        .args([
            "-y",
            "-ss",
            seek,
            "-i",
            &video.to_string_lossy(),
            "-frames:v",
            "1",
            &output.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn extract_video_frame_hash(video: &Path, ffmpeg: &Path) -> eyre::Result<ImageHash> {
    let tmp = tempfile::NamedTempFile::new()?.into_temp_path();
    let frame_path = tmp.to_path_buf().with_extension("png");

    for seek in &["1", "0"] {
        if run_ffmpeg_extract(ffmpeg, video, seek, &frame_path)
            && frame_path.exists()
            && std::fs::metadata(&frame_path)?.len() > 0
        {
            let img = image::open(&frame_path)
                .map_err(|e| eyre::eyre!("failed to open extracted frame: {e}"))?;
            let _ = std::fs::remove_file(&frame_path);
            return Ok(hasher().hash_image(&img));
        }
    }

    let _ = std::fs::remove_file(&frame_path);
    Err(eyre::eyre!(
        "ffmpeg could not extract frame from {}",
        video.display()
    ))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use image::{ImageBuffer, Rgb, RgbImage};

    use super::*;

    fn create_gradient_image(path: &PathBuf, horizontal: bool) {
        let img: RgbImage = ImageBuffer::from_fn(100, 100, |x, y| {
            let val = if horizontal { x as u8 } else { y as u8 };
            Rgb([val, val, val])
        });
        img.save(path).unwrap();
    }

    fn create_checkerboard_image(path: &PathBuf, block_size: u32) {
        let img: RgbImage = ImageBuffer::from_fn(100, 100, |x, y| {
            if ((x / block_size) + (y / block_size)) % 2 == 0 {
                Rgb([255, 255, 255])
            } else {
                Rgb([0, 0, 0])
            }
        });
        img.save(path).unwrap();
    }

    #[test]
    fn identical_images_same_hash() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        create_gradient_image(&a, true);
        create_gradient_image(&b, true);

        let hash_a = compute_image_hash(&a).unwrap();
        let hash_b = compute_image_hash(&b).unwrap();
        assert_eq!(hash_a.dist(&hash_b), 0);
    }

    #[test]
    fn different_images_different_hash() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        create_gradient_image(&a, true);
        create_checkerboard_image(&b, 10);

        let hash_a = compute_image_hash(&a).unwrap();
        let hash_b = compute_image_hash(&b).unwrap();
        assert!(hash_a.dist(&hash_b) > 0);
    }

    #[test]
    fn nonexistent_file_errors() {
        let result = compute_image_hash(Path::new("/nonexistent.png"));
        assert!(result.is_err());
    }

    #[test]
    fn ffmpeg_not_at_path_errors() {
        let result = extract_video_frame_hash(
            Path::new("/nonexistent.mp4"),
            Path::new("/nonexistent/ffmpeg"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn find_ffmpeg_succeeds_when_installed() {
        if which::which("ffmpeg").is_ok() {
            assert!(find_ffmpeg().is_ok());
        }
    }
}
