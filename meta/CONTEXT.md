# Dublette

Dublette deduplicates images and videos using perceptual hashing. The codebase has one bounded context — the deduplication of media collections under one or more directories.

## Language

**Deduplication**:
The pipeline that discovers media files in given directories, computes perceptual hashes, groups visually similar files, and removes all but one file per group.

**Media file**:
A file whose extension matches the supported image set (jpg, jpeg, png, bmp, gif, tiff, webp) or video set (mp4, mov, avi, mkv, wmv, flv, webm, m4v, 3gp).
_Avoid_: file, photo, image (when ambiguous between media kind and a literal image file).

**Media kind**:
Whether a media file is treated as an image or a video. Images are hashed directly; videos are hashed by extracting a single frame via ffmpeg.

**Perceptual hash**:
A fixed-size fingerprint computed from a media file's visual content. Close hashes imply visually similar files even when bytes differ.
_Avoid_: hash (ambiguous with cryptographic hash), fingerprint (unless qualified).

**Hamming distance**:
The number of differing bits between two perceptual hashes. The metric for "how visually different are these two files".
_Avoid_: distance, similarity score.

**Threshold**:
The maximum hamming distance at which two media files are treated as duplicates of each other. Configured per run.
_Avoid_: tolerance, sensitivity.

**Duplicate group**:
A cluster of media files connected by pairwise hamming distance ≤ threshold (transitive closure). One file is kept (alphabetically first); the rest are flagged for deletion.
_Avoid_: cluster, set, batch.

**Empty file**:
A zero-byte media file. Found independently of the deduplication pipeline; deleted only when explicitly requested by the caller.

**Skipped file**:
A media file that could not be hashed (corrupt image, ffmpeg failure, unreadable). Recorded as data in the deduplication output, not silently swallowed.

## Relationships

- A **Media file** has at most one **Perceptual hash** per pipeline run; un-hashable files become **Skipped files**.
- A **Duplicate group** contains two or more **Media files** of the same **Media kind** (image or video; never mixed across kinds).
- The **Hamming distance** between two **Perceptual hashes** is the deciding metric for membership in a **Duplicate group**.
- **Empty files** and **Skipped files** are not part of any **Duplicate group** but appear in the **Deduplication** output.

## Example dialogue

> **Dev:** "Should two visually identical files — one image, one video — end up in the same **Duplicate group**?"
> **Maintainer:** "No — **Duplicate groups** are kind-pure. The image and video pipelines are independent passes; their **Perceptual hashes** are not compared across **Media kinds**."

## Flagged ambiguities

- "duplicate" was used loosely to mean both byte-identical and visually-similar — resolved: in this project, **Duplicate** always means within-**Threshold** by **Hamming distance** on the **Perceptual hash**, never byte-equality.
