# CLI Reference

## Usage

```bash
dublette <DIRECTORY> [OPTIONS]
```

The `DIRECTORY` argument is required. Dublette scans it recursively for media files.

## Options

| Flag | Long             | Type                 | Default | Description                                                  |
| ---- | ---------------- | -------------------- | ------- | ------------------------------------------------------------ |
| `-t` | `--threshold`    | integer              | `1`     | Maximum hamming distance to consider two files as duplicates |
| `-n` | `--dry-run`      | flag                 | `false` | List duplicates without deleting any files                   |
|      | `--only`         | `images` or `videos` | both    | Restrict processing to one media type                        |
|      | `--delete-empty` | flag                 | `false` | Find and delete 0-byte media files                           |
| `-y` | `--yes`          | flag                 | `false` | Skip the confirmation prompt before deletion                 |
| `-q` | `--quiet`        | flag                 | `false` | Suppress progress bars and scanning messages                 |
| `-v` | `--verbose`      | flag                 | `false` | Print per-file hashes and pairwise distances                 |
|      | `--no-color`     | flag                 | `false` | Disable colored terminal output                              |
|      | `--json`         | flag                 | `false` | Output results as JSON instead of a table                    |
| `-h` | `--help`         | flag                 |         | Print help information                                       |
| `-V` | `--version`      | flag                 |         | Print version                                                |

## Option Details

### `--threshold` (`-t`)

Controls the maximum hamming distance between two perceptual hashes for them to be considered duplicates. A distance of `0` means the hashes must be identical. The default of `1` tolerates a single bit of difference, which catches files that are visually the same but differ slightly from re-encoding.

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

Restrict processing to images or videos only. Without this flag, both are processed.

```bash
dublette ~/Media --only images
dublette ~/Media --only videos
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

| Code  | Meaning                                                                 |
| ----- | ----------------------------------------------------------------------- |
| `0`   | Success (no duplicates found, or duplicates deleted successfully)       |
| `1`   | Dry-run found duplicates                                                |
| `2`   | Invalid argument (missing directory, nonexistent path, not a directory) |
| `130` | Interrupted by Ctrl+C                                                   |

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
  "dry_run": false
}
```

| Field              | Type             | Description                                                           |
| ------------------ | ---------------- | --------------------------------------------------------------------- |
| `empty_files`      | array of strings | Relative paths of 0-byte files (only populated with `--delete-empty`) |
| `groups`           | array of objects | Each group contains a `keep` path and a `duplicates` array            |
| `total_duplicates` | integer          | Total number of files marked for deletion                             |
| `dry_run`          | boolean          | Whether this was a dry run                                            |

## Supported File Formats

### Images

jpg, jpeg, png, bmp, gif, tiff, webp

### Videos (requires ffmpeg)

mp4, mov, avi, mkv, wmv, flv, webm, m4v, 3gp

File extension matching is case-insensitive. A file named `PHOTO.JPG` is treated the same as `photo.jpg`.
