# Deduplication Process

This page describes what happens internally when you run `dublette <DIRECTORY>...`.

## Step 1: Handle Empty Files (Optional)

If `--delete-empty` is set, dublette first scans for 0-byte media files (matched by extension). These are reported and deleted (or listed in dry-run mode) before deduplication begins.

This step is independent of hashing. A 0-byte file cannot be hashed and is always skipped during the normal scan.

## Step 2: Collect Files

Dublette walks each directory tree recursively using `walkdir`. For each file:

1. Check that it is a regular file (not a directory or symlink target without content)
2. Extract the file extension and lowercase it for case-insensitive matching
3. Match against supported extensions (images: jpg, jpeg, png, bmp, gif, tiff, webp; videos: mp4, mov, avi, mkv, wmv, flv, webm, m4v, 3gp; audio: mp3, flac, ogg, opus, m4a, aac, wav, wma, aiff)
4. Skip files with 0 bytes

The `--only` flag restricts which extension set is used. Without it, image, video, and audio extensions are processed in separate passes.

The resulting file list is sorted alphabetically. This deterministic ordering ensures consistent results across runs.

## Step 3: Hash Files in Parallel

Each file is hashed using the DoubleGradient perceptual hashing algorithm (see [Perceptual Hashing](perceptual-hashing.md)).

Hashing runs in parallel across all available CPU cores using `rayon`. Files that fail to hash (corrupted, unsupported codec) are skipped with a warning.

### Image Hashing

The image is opened, decoded, and passed to the `image_hasher` hasher, which produces a 40-bit perceptual hash.

### Video Hashing

Videos require an extra step:

1. ffmpeg extracts a single frame from the video at the **1-second mark**
2. If extraction at 1s fails (e.g., the video is shorter), it retries at **0 seconds**
3. The extracted frame is saved as a temporary PNG
4. The PNG is hashed using the same image hashing pipeline
5. The temporary file is cleaned up

This means video deduplication compares a representative frame, not the full video stream. Videos that share the same opening frame (within the threshold) are considered duplicates.

If ffmpeg is not installed, video processing is skipped entirely with a warning.

### Audio Fingerprinting (recording match)

Under the default `--audio-match recording`, ffmpeg decodes up to the first 120 seconds of each audio file to mono PCM, which is piped in-process to a Chromaprint-class fingerprinter. The resulting acoustic fingerprint captures what the audio sounds like, independent of codec, bitrate, or tags. Audio tracks embedded in video files are never fingerprinted.

If ffmpeg is not installed, the audio pass is skipped with a warning (encoding match still works).

### Audio Stream Hashing (encoding match)

Under `--audio-match encoding`, each file is copied to a temporary location, all tags are stripped, and the remaining encoded stream is hashed. Files group only on exact stream equality: a retagged copy matches, a re-encode never does. No decoding happens, so ffmpeg is not required.

## Step 4: Pairwise Comparison

Every pair of hashes is compared. For images and videos the metric is hamming distance against `--threshold`; for audio recording match it is normalized fingerprint dissimilarity (0.0-1.0) against `--audio-threshold`. This is an O(n^2) operation over the number of files. Encoding-match audio skips this step and groups by hash equality directly.

For each pair where the distance is at or below the threshold, both files are recorded as potential duplicates of each other. This produces a bidirectional adjacency map.

With `--verbose`, the distance for every pair is printed to stderr.

## Step 5: Build Duplicate Groups

The adjacency map is converted into transitive groups using depth-first search. If A matches B and B matches C, then A, B, and C are placed in the same group -- even if A and C do not directly match.

Within each group, files are sorted alphabetically. The first file is designated as the one to **keep**; the rest are marked for deletion. Recording-match audio groups are the exception: because they deliberately group files of unequal fidelity, the kept file is the highest-fidelity member -- lossless (flac, wav, aiff) over lossy, then higher bitrate, then alphabetical tiebreak.

## Step 6: Report Results

Depending on the output mode:

- **Table mode** (default): A formatted table is printed to stdout showing each group, which file is kept, and which are marked for deletion.
- **JSON mode** (`--json`): A JSON object is printed to stdout with the group structure, empty file list, and dry-run status.

## Step 7: Delete Duplicates

If this is not a dry run and duplicates were found:

1. The list of files to delete uses paths relative to the current working directory
2. A confirmation prompt is shown (unless `-y` is set)
3. Each file is removed from disk
4. A summary of deleted files is printed to stderr

In dry-run mode, this step is skipped entirely and the exit code is set to `1` if any duplicates were found.

## Processing Order

Images, videos, and audio are processed in separate passes, in that order. Each pass produces its own set of duplicate groups; groups never mix media kinds. The groups are merged for JSON output but displayed separately in table mode.
