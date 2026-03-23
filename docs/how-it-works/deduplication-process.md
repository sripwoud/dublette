# Deduplication Process

This page describes what happens internally when you run `dublette <DIRECTORY>`.

## Step 1: Handle Empty Files (Optional)

If `--delete-empty` is set, dublette first scans for 0-byte media files (matched by extension). These are reported and deleted (or listed in dry-run mode) before deduplication begins.

This step is independent of hashing. A 0-byte file cannot be hashed and is always skipped during the normal scan.

## Step 2: Collect Files

Dublette walks the directory tree recursively using `walkdir`. For each file:

1. Check that it is a regular file (not a directory or symlink target without content)
2. Extract the file extension and lowercase it for case-insensitive matching
3. Match against supported extensions (images: jpg, jpeg, png, bmp, gif, tiff, webp; videos: mp4, mov, avi, mkv, wmv, flv, webm, m4v, 3gp)
4. Skip files with 0 bytes

The `--only` flag restricts which extension set is used. Without it, both image and video extensions are processed in separate passes.

The resulting file list is sorted alphabetically. This deterministic ordering ensures consistent results across runs.

## Step 3: Hash Files in Parallel

Each file is hashed using the DoubleGradient perceptual hashing algorithm (see [Perceptual Hashing](perceptual-hashing.md)).

Hashing runs in parallel across all available CPU cores using `rayon`. Files that fail to hash (corrupted, unsupported codec) are skipped with a warning.

### Image Hashing

The image is opened, decoded, and passed to the `img_hash` hasher, which produces a 128-bit perceptual hash.

### Video Hashing

Videos require an extra step:

1. ffmpeg extracts a single frame from the video at the **1-second mark**
2. If extraction at 1s fails (e.g., the video is shorter), it retries at **0 seconds**
3. The extracted frame is saved as a temporary PNG
4. The PNG is hashed using the same image hashing pipeline
5. The temporary file is cleaned up

This means video deduplication compares a representative frame, not the full video stream. Videos that share the same opening frame (within the threshold) are considered duplicates.

If ffmpeg is not installed, video processing is skipped entirely with a warning.

## Step 4: Pairwise Comparison

Every pair of hashes is compared using hamming distance. This is an O(n^2) operation over the number of files.

For each pair where the distance is at or below the `--threshold`, both files are recorded as potential duplicates of each other. This produces a bidirectional adjacency map.

With `--verbose`, the distance for every pair is printed to stderr.

## Step 5: Build Duplicate Groups

The adjacency map is converted into transitive groups using depth-first search. If A matches B and B matches C, then A, B, and C are placed in the same group -- even if A and C do not directly match.

Within each group, files are sorted alphabetically. The first file is designated as the one to **keep**; the rest are marked for deletion.

## Step 6: Report Results

Depending on the output mode:

- **Table mode** (default): A formatted table is printed to stdout showing each group, which file is kept, and which are marked for deletion.
- **JSON mode** (`--json`): A JSON object is printed to stdout with the group structure, empty file list, and dry-run status.

## Step 7: Delete Duplicates

If this is not a dry run and duplicates were found:

1. The list of files to delete is resolved relative to the scanned directory
2. A confirmation prompt is shown (unless `-y` is set)
3. Each file is removed from disk
4. A summary of deleted files is printed to stderr

In dry-run mode, this step is skipped entirely and the exit code is set to `1` if any duplicates were found.

## Processing Order

Images and videos are processed in separate passes. Image deduplication runs first, followed by video deduplication. Each pass produces its own set of duplicate groups. The groups are merged for JSON output but displayed separately in table mode.
