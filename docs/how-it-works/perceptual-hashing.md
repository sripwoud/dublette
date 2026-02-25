# Perceptual Hashing

## The Problem with Byte-Level Comparison

Traditional file comparison (checksums like MD5 or SHA-256) produces completely different hashes for files that differ by even a single byte. This means two photos that look identical to the human eye -- but differ in EXIF metadata, compression level, or format -- will have entirely different checksums.

Perceptual hashing solves this by generating hashes based on the visual content of an image, not its raw bytes.

## What Is a Perceptual Hash?

A perceptual hash reduces an image to a compact fingerprint that captures its visual structure. Two images that look similar produce similar hashes, even if they differ in:

- File format (e.g., a JPEG and a PNG of the same photo)
- Compression quality
- Minor cropping or resizing
- EXIF metadata (camera info, timestamps)
- Color depth

The hash captures the overall structure -- gradients, edges, brightness patterns -- rather than pixel-exact data.

## DoubleGradient Algorithm

Dublette uses the **DoubleGradient** algorithm from the `img_hash` crate with an 8x8 hash size.

DoubleGradient works by:

1. Resizing the image to a small grid (9x8 for horizontal gradients, 8x9 for vertical)
2. Computing brightness gradients in both horizontal and vertical directions
3. Encoding whether each gradient increases or decreases as a single bit

This produces a 128-bit hash (8x8 horizontal + 8x8 vertical = 128 gradient comparisons). The result is a compact binary fingerprint that is robust to scaling, minor color shifts, and compression artifacts.

DoubleGradient was chosen over alternatives (Mean, Gradient, Blockhash) because it provides a good balance of accuracy and resistance to false positives. It captures directional structure in the image, making it more discriminating than simpler averaging methods.

## Hamming Distance

Two perceptual hashes are compared using **hamming distance**: the number of bit positions where the hashes differ.

- Distance `0` -- hashes are identical; the images have the same visual structure
- Distance `1` -- one bit differs; the images are nearly identical
- Distance `5` -- several bits differ; the images share broad similarity
- Distance `30+` -- the images are visually different

## The Threshold

The `--threshold` flag sets the maximum hamming distance at which two files are considered duplicates.

| Threshold     | Behavior                                                                                        |
| ------------- | ----------------------------------------------------------------------------------------------- |
| `0`           | Exact perceptual match only. Catches re-encoded copies and format conversions.                  |
| `1` (default) | Allows one bit of difference. Catches slight compression variations.                            |
| `2-3`         | More lenient. May catch images with minor edits (slight crop, brightness adjustment).           |
| `5+`          | Aggressive. Higher risk of false positives -- visually distinct images may be grouped together. |

Start with the default of `1` and use `--dry-run` to verify results before increasing.

## Limitations

Perceptual hashing is not perfect. It may produce false positives or false negatives in certain cases:

- **Heavily cropped images** may hash differently if the crop changes the dominant visual structure
- **Images with overlaid text or watermarks** may not match their clean originals
- **Very small images** (thumbnails) produce less discriminating hashes
- **Completely different images with coincidentally similar gradient patterns** could match at high thresholds

Always use `--dry-run` to review results before deleting files.
