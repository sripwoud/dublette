# Dublette

Deduplicate images, videos, and audio using perceptual hashing and acoustic fingerprints.

Dublette scans a directory for similar media files and removes the duplicates. Unlike byte-level comparison, it uses perceptual hashing to detect files that look the same and acoustic fingerprints to detect files that sound the same, even when they differ in format, compression, or metadata.

## Key Features

- **Perceptual hashing** -- detects visually similar images and videos, not just byte-identical copies
- **Acoustic fingerprinting** -- groups the same recording across mp3, flac, and 7 more audio formats, keeping the highest-fidelity copy
- **Image and video support** -- handles jpg, png, gif, webp, bmp, tiff, and 9 video formats via ffmpeg
- **Dry-run mode** -- preview what would be deleted before committing
- **JSON output** -- machine-readable output for scripting and CI pipelines
- **Parallel processing** -- hashes files concurrently using all available cores
- **Configurable threshold** -- tune sensitivity with hamming distance control

## Quick Example

Preview duplicates without deleting anything:

```bash
dublette ~/Photos --dry-run
```

Delete duplicates, skipping the confirmation prompt:

```bash
dublette ~/Photos --yes
```

Output results as JSON for scripting:

```bash
dublette ~/Photos --dry-run --json
```

## Getting Started

See [Installation](getting-started/installation.md) to install dublette, then follow the [Quick Start](getting-started/quick-start.md) guide.
