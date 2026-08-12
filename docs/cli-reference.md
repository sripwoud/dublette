# CLI Reference

## Usage

```bash
dublette <DIRECTORY>... [OPTIONS]
```

One or more directories are required. Dublette scans all of them recursively and detects
duplicates across directories as well as within them.

```bash
dublette ~/Photos
dublette 2020 2021 2022
dublette ~/Photos ~/Backup/Photos
```

Shell glob expansion works naturally — `dublette 202*` expands to all matching directories
before dublette runs, so no special pattern syntax is needed.

## Options

| Flag | Long                | Type                           | Default     | Description                                                              |
| ---- | ------------------- | ------------------------------ | ----------- | ------------------------------------------------------------------------ |
| `-t` | `--threshold`       | integer                        | `1`         | Maximum hamming distance to consider two image/video files as duplicates |
| `-n` | `--dry-run`         | flag                           | `false`     | List duplicates without deleting any files                               |
|      | `--only`            | `images`, `videos`, or `audio` | all         | Restrict processing to one media type                                    |
|      | `--audio-match`     | `recording` or `encoding`      | `recording` | How audio files are matched                                              |
|      | `--audio-threshold` | float (0.0-1.0)                | `0.1`       | Maximum acoustic fingerprint dissimilarity; recording match only         |
|      | `--delete-empty`    | flag                           | `false`     | Find and delete 0-byte media files                                       |
| `-y` | `--yes`             | flag                           | `false`     | Skip the confirmation prompt before deletion                             |
| `-q` | `--quiet`           | flag                           | `false`     | Suppress progress bars and scanning messages                             |
| `-v` | `--verbose`         | flag                           | `false`     | Print per-file hashes and pairwise distances                             |
|      | `--no-color`        | flag                           | `false`     | Disable colored terminal output                                          |
|      | `--json`            | flag                           | `false`     | Output results as JSON instead of a table                                |
| `-h` | `--help`            | flag                           |             | Print help information                                                   |
| `-V` | `--version`         | flag                           |             | Print version                                                            |

## Option Details

### `--threshold` (`-t`)

Controls the maximum hamming distance between two perceptual hashes for image and video files to be considered duplicates. A distance of `0` means the hashes must be identical. The default of `1` tolerates a single bit of difference, which catches files that are visually the same but differ slightly from re-encoding. Audio has its own threshold (`--audio-threshold`).

Higher values catch more aggressively similar files but increase the risk of false positives.

```bash
dublette ~/Photos -t 0 -n
dublette ~/Photos -t 3 -n
```

### `--dry-run` (`-n`)

Shows what would be deleted without actually removing any files. Useful for previewing results before committing. The exit code is `1` when duplicates are found, enabling use in scripts.

```bash
dublette ~/Photos -n
```

### `--only`

Restrict processing to one media type. Without this flag, images, videos, and audio are all processed in separate passes.

```bash
dublette ~/Media --only images
dublette ~/Media --only videos
dublette ~/Media --only audio
```

### `--audio-match`

Selects how audio files are compared. Both strategies are explained in [Acoustic Fingerprinting](how-it-works/acoustic-fingerprinting.md).

- `recording` (default): computes an acoustic fingerprint from the decoded audio (up to the first 120 seconds, via ffmpeg) and groups files whose fingerprint dissimilarity is within `--audio-threshold`. The same recording matches across formats and bitrates -- an mp3 rip of a flac is a duplicate.
- `encoding`: hashes the encoded audio stream with tag regions excluded. A retagged copy matches; a re-encode never does. Exact, takes no threshold, and needs no ffmpeg.

Recording-match groups keep the highest-fidelity file: lossless (flac, wav, aiff) over lossy, then higher bitrate, then alphabetical. All other groups keep the alphabetically first file.

```bash
dublette ~/Music --audio-match encoding -n
```

### `--audio-threshold`

Maximum normalized dissimilarity (0.0-1.0) between two acoustic fingerprints for the files to be considered the same recording. Applies to recording match only; combining it with `--audio-match encoding` is an error. The default of `0.1` comfortably matches the same recording across codecs while rejecting different recordings. See [Acoustic Fingerprinting](how-it-works/acoustic-fingerprinting.md) for how dissimilarity is computed and a per-value tuning table.

```bash
dublette ~/Music --audio-threshold 0.05 -n
```

### `--delete-empty`

Scans for 0-byte media files and deletes them before deduplication. This is a separate step from duplicate detection. Empty files are identified by extension (same set as normal processing).

```bash
dublette ~/Photos --delete-empty -y
```

### `--yes` (`-y`)

Skips the interactive confirmation prompt before deleting files. Required for non-interactive environments (scripts, CI).

```bash
dublette ~/Photos -y
```

### `--quiet` (`-q`)

Suppresses progress bars and status messages written to stderr. Table output and JSON output are unaffected.

```bash
dublette ~/Photos -q -n
```

### `--verbose` (`-v`)

Prints the computed hash for each file and the hamming distance for every pairwise comparison. Output goes to stderr.

```bash
dublette ~/Photos -v -n
```

### `--json`

Outputs results as a JSON object to stdout instead of a table. Suppresses table output. See [JSON Output Format](#json-output-format) below.

```bash
dublette ~/Photos -n --json
```

### `--no-color`

Disables colored terminal output.

```bash
dublette ~/Photos --no-color -n
```

## Exit Codes

| Code  | Meaning                                                                   |
| ----- | ------------------------------------------------------------------------- |
| `0`   | Success (no duplicates found, or duplicates deleted successfully)         |
| `1`   | Dry-run found duplicates                                                  |
| `2`   | Invalid argument (missing directories, nonexistent path, not a directory) |
| `130` | Interrupted by Ctrl+C                                                     |

The exit code `1` in dry-run mode is intentional: it allows scripts to detect whether duplicates exist without deleting them.

## JSON Output Format

When `--json` is used, stdout contains a single JSON object:

```json
{
  "empty_files": ["path/to/empty.jpg"],
  "groups": [
    {
      "keep": "photos/original.jpg",
      "duplicates": ["photos/copy.jpg", "photos/another-copy.jpg"]
    }
  ],
  "total_duplicates": 2,
  "dry_run": false,
  "skipped": [
    {
      "path": "music/corrupt.mp3",
      "reason": "decode_failed",
      "detail": "ffmpeg could not decode audio from music/corrupt.mp3"
    }
  ],
  "total_skipped": 1,
  "warnings": ["ffmpeg not found on PATH; skipping video pass"]
}
```

| Field              | Type             | Description                                                             |
| ------------------ | ---------------- | ----------------------------------------------------------------------- |
| `empty_files`      | array of strings | Relative paths of 0-byte files (only populated with `--delete-empty`)   |
| `groups`           | array of objects | Each group contains a `keep` path and a `duplicates` array              |
| `total_duplicates` | integer          | Total number of files marked for deletion                               |
| `dry_run`          | boolean          | Whether this was a dry run                                              |
| `skipped`          | array of objects | Files that could not be hashed; each has `path`, `reason`, and `detail` |
| `total_skipped`    | integer          | Number of entries in `skipped`                                          |
| `warnings`         | array of strings | Pass-level warnings, such as an entire media pass being skipped         |

All seven fields are always present. `skipped` and `warnings` are `[]` and `total_skipped` is `0` when a run processes every file.

Skipped files are the reason a result is a floor, not a total: a file that was never fingerprinted is a file whose duplicate cannot have been found. The same entries are also printed to stderr as `Warning: skipping <path>: <detail>` regardless of `--json`.

### Skip Reasons

`reason` is a stable tag safe to branch on. `detail` is the human-readable message and may change between releases.

| Tag                     | Meaning                                                                                                                     |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `decode_failed`         | The file's content could not be decoded -- corrupt image, ffmpeg decode failure, no usable video frame. Worth investigating |
| `too_short`             | Audio decoded but was too short (under ~3 seconds) to yield an acoustic fingerprint                                         |
| `unsupported_container` | No tag parser understands the container. Expected for wma under `--audio-match encoding`                                    |
| `unreadable`            | The file could not be read at all -- permissions, I/O error, or a failed tag-region exclusion                               |

New tags may be added in future releases; a consumer matching on `reason` should tolerate unknown values.

## Supported File Formats

### Images

jpg, jpeg, png, bmp, gif, tiff, webp

### Videos (requires ffmpeg)

mp4, mov, avi, mkv, wmv, flv, webm, m4v, 3gp

### Audio (recording match requires ffmpeg)

mp3, flac, ogg, opus, m4a, aac, wav, wma, aiff

Audio tracks embedded in video files are never fingerprinted -- a music video and its audio rip are not duplicates of each other. Under encoding match, wma files are reported as skipped (no tag parser support); recording match handles them via ffmpeg.

File extension matching is case-insensitive. A file named `PHOTO.JPG` is treated the same as `photo.jpg`.
