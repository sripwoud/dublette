<p align="center">
  <h1 align="center"><a href="https://Dublette.sripwoud.xyz">Auberge</a></h1>
</p>
<p align="center">
  <a href="https://crates.io/crates/dublette">
    <img src="https://img.shields.io/crates/v/dublette" alt="Crates.io">
  </a>
</p>

> Deduplicate images and videos using perceptual hashing

Dublette scans a directory for visually similar media files and removes the duplicates. Unlike byte-level comparison, it uses perceptual hashing to detect files that look the same even if they differ in format, compression, or metadata.

## Features

- **Perceptual hashing** -- detects visually similar images and videos, not just byte-identical copies
- **Image and video support** -- handles jpg, png, gif, webp, bmp, tiff, and 9 video formats via ffmpeg
- **Dry-run mode** -- preview what would be deleted before committing
- **JSON output** -- machine-readable output for scripting and CI pipelines
- **Parallel processing** -- hashes files concurrently using all available cores
- **Configurable threshold** -- tune sensitivity with hamming distance control

## Quick Start

Install dublette:

```bash
cargo install dublette
```

Preview duplicates:

```bash
dublette ~/Photos --dry-run
```

Delete duplicates:

```bash
dublette ~/Photos --yes
```

## Documentation

Full documentation available at [dublette.sripwoud.xyz](https://dublette.sripwoud.xyz):

- [Installation](https://dublette.sripwoud.xyz/#/getting-started/installation) - Detailed setup guide
- [Quick Start](https://dublette.sripwoud.xyz/#/getting-started/quick-start) - Step-by-step walkthrough
- [CLI Reference](https://dublette.sripwoud.xyz/#/cli-reference) - All options documented
- [How It Works](https://dublette.sripwoud.xyz/#/how-it-works/perceptual-hashing) - Perceptual hashing explained

## Requirements

- Rust/Cargo for installation
- (Optional) ffmpeg for video deduplication

## Community

- [Documentation](https://dublette.sripwoud.xyz)
- [Report Issues](https://github.com/sripwoud/dublette/issues)
