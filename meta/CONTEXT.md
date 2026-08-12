# Dublette

Dublette deduplicates images and videos using perceptual hashing. The codebase has one bounded context — the deduplication of media collections under one or more directories.

## Language

**Deduplication**:
The pipeline that discovers media files in given directories, computes perceptual hashes, groups visually similar files, and removes all but one file per group.

**Media file**:
A file whose extension matches the supported image set (jpg, jpeg, png, bmp, gif, tiff, webp) or video set (mp4, mov, avi, mkv, wmv, flv, webm, m4v, 3gp).
_Avoid_: file, photo, image (when ambiguous between media kind and a literal image file).

**Media kind**:
Whether a media file is treated as an image, a video, or an audio track. Images are hashed directly; videos are hashed by extracting a single frame via ffmpeg; audio tracks are compared per the audio match strategy.

**Perceptual hash**:
A fixed-size fingerprint computed from a media file's visual content. Close hashes imply visually similar files even when bytes differ.
_Avoid_: hash (ambiguous with cryptographic hash), fingerprint (unless qualified).

**Acoustic fingerprint**:
A fingerprint computed from a media file's decoded audio content (Chromaprint-class). Close fingerprints imply the same recording even when encoding, bitrate, or tags differ. The audio counterpart of the perceptual hash.
_Avoid_: audio hash, perceptual hash (reserved for visual content).

**Match strategy** (audio only):
How audio files are compared for duplication. **Recording match** (default): acoustic fingerprints within threshold — same recording regardless of encoding. **Encoding match** (strict): a content hash of the encoded audio stream with tag/metadata regions excluded — "same file, retagged" matches; a re-encode never does. Motivated by deletion safety (never group a lossless file with a lossy re-encode by accident) and by cost (no decoding, I/O-bound). Encoding match finds a strict subset of what recording match finds.

**Hamming distance**:
The number of differing bits between two perceptual hashes. The metric for "how visually different are these two files".
_Avoid_: distance, similarity score.

**Threshold**:
The per-media-kind maximum distance at which two media files are treated as duplicates of each other. Image/video: hamming distance in bits on the perceptual hash (one shared value). Audio: normalized dissimilarity (0.0–1.0) between acoustic fingerprints; applies only under recording match — encoding match is exact and takes no threshold. Configured per run.
_Avoid_: tolerance, sensitivity.

**Duplicate group**:
A cluster of media files connected by pairwise distance ≤ threshold (transitive closure). One file is kept per the keep policy; the rest are flagged for deletion.
_Avoid_: cluster, set, batch.

**Keep policy**:
How the kept file of a duplicate group is chosen. Image, video, and encoding-match audio groups: alphabetically first. Recording-match audio groups: highest fidelity — lossless over lossy, then higher bitrate, then alphabetical tiebreak. Recording match deliberately groups files of unequal quality, so alphabetical order alone could silently keep a lossy copy over a lossless one.

**Empty file**:
A zero-byte media file. Found independently of the deduplication pipeline; deleted only when explicitly requested by the caller.

**Skipped file**:
A media file that could not be hashed (corrupt image, ffmpeg failure, unreadable). Recorded as data in the deduplication output, not silently swallowed.

## Relationships

- A **Media file** has at most one **Perceptual hash** per pipeline run; un-hashable files become **Skipped files**.
- A **Duplicate group** contains two or more **Media files** of the same **Media kind** (image, video, or audio; never mixed across kinds). Audio streams embedded in video files are never fingerprinted — a music video and its audio rip are not duplicates of each other.
- The **Hamming distance** between two **Perceptual hashes** is the deciding metric for membership in a **Duplicate group**.
- **Empty files** and **Skipped files** are not part of any **Duplicate group** but appear in the **Deduplication** output.

## Example dialogue

> **Dev:** "Should two visually identical files — one image, one video — end up in the same **Duplicate group**?"
> **Maintainer:** "No — **Duplicate groups** are kind-pure. The image and video pipelines are independent passes; their **Perceptual hashes** are not compared across **Media kinds**."

## Flagged ambiguities

- "duplicate" was used loosely to mean both byte-identical and visually-similar — resolved: in this project, **Duplicate** always means within-**Threshold** by **Hamming distance** on the **Perceptual hash**, never byte-equality. For audio, "duplicate" is defined by the active **Match strategy**; even **Encoding match** is exactness on audio content, not file byte-equality (tags may differ).
- The mechanism of **Encoding match** was debated (stream bytes minus tags vs decoded-PCM hash) — resolved: stream bytes minus tags. Decoded-PCM would require full decode, making "strict" slower than **Recording match** and killing the cost motive; lossless↔lossless transcodes (FLAC↔WAV) are intentionally NOT encoding matches — they are recording matches.
