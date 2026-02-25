use std::path::Path;

use img_hash::{HashAlg, HasherConfig, ImageHash};

pub fn compute_image_hash(path: &Path) -> eyre::Result<ImageHash> {
    let img =
        image::open(path).map_err(|e| eyre::eyre!("failed to open {}: {e}", path.display()))?;
    let hasher = HasherConfig::new()
        .hash_alg(HashAlg::DoubleGradient)
        .hash_size(8, 8)
        .to_hasher();
    Ok(hasher.hash_image(&img))
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
}
