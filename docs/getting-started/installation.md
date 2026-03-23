# Installation

## Prerequisites

- **ffmpeg** -- optional, required only for video deduplication

### Installing ffmpeg

Video processing requires ffmpeg to be available on your `PATH`. If you only need image deduplication, you can skip this.

**macOS:**

```bash
brew install ffmpeg
```

**Ubuntu / Debian:**

```bash
sudo apt install ffmpeg
```

**Arch Linux:**

```bash
sudo pacman -S ffmpeg
```

**Windows:**

Download from [ffmpeg.org](https://ffmpeg.org/download.html) and add to your `PATH`.

Without ffmpeg, dublette will skip video files and print a warning.

## Pre-compiled binaries

Download a binary for your platform from the [latest release](https://github.com/sripwoud/dublette/releases/latest). Binaries are available for Linux (x86_64, aarch64), macOS (Intel, Apple Silicon), and Windows (x64).

Extract it somewhere on your `PATH`, e.g. `~/.local/bin`.

## Install from crates.io

```bash
cargo install dublette
```

This installs the `dublette` binary to `~/.cargo/bin/`. Make sure this directory is in your `PATH`.

## Build from Source

```bash
git clone https://github.com/sripwoud/dublette.git
cd dublette
cargo build --release
```

The binary will be at `target/release/dublette`.

## Verify Installation

```bash
dublette --version
```

This should print the version number. You can also run `dublette --help` to see all available options.
