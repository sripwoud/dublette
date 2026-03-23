# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1](https://github.com/sripwoud/dublette/compare/v0.2.0...v0.2.1) - 2026-03-23

### Fixed

- deduplicate empty file paths before deletion ([#24](https://github.com/sripwoud/dublette/pull/24))
- *(mise)* use `cargo build --release` in build task ([#23](https://github.com/sripwoud/dublette/pull/23))

### Other

- update deduplication-process.md for multi-directory support ([#25](https://github.com/sripwoud/dublette/pull/25))

## [0.2.0](https://github.com/sripwoud/dublette/compare/v0.1.7...v0.2.0) - 2026-03-23

### Added

- accept multiple directories as positional arguments ([#12](https://github.com/sripwoud/dublette/pull/12))

## [0.1.7](https://github.com/sripwoud/dublette/compare/v0.1.6...v0.1.7) - 2026-02-27

### Added

- *(ci)* add sha256 checksums to release binaries

## [0.1.6](https://github.com/sripwoud/dublette/compare/v0.1.5...v0.1.6) - 2026-02-26

### Fixed

- *(ci)* drop x86_64-apple-darwin target (macos intel runners deprecated)

## [0.1.5](https://github.com/sripwoud/dublette/compare/v0.1.4...v0.1.5) - 2026-02-26

### Fixed

- *(ci)* replace deprecated macos-13 runner with macos-15-large

## [0.1.4](https://github.com/sripwoud/dublette/compare/v0.1.3...v0.1.4) - 2026-02-26

### Fixed

- *(ci)* use correct release-plz output field for tag extraction

## [0.1.3](https://github.com/sripwoud/dublette/compare/v0.1.2...v0.1.3) - 2026-02-26

### Fixed

- *(ci)* pass release tag to binary upload action

## [0.1.2](https://github.com/sripwoud/dublette/compare/v0.1.1...v0.1.2) - 2026-02-26

### Fixed

- *(docs)* update title in main README

### Other

- add pre-compiled binary install option ([#6](https://github.com/sripwoud/dublette/pull/6))
- add workflow_dispatch for manual binary builds
- add multi-platform binary releases ([#5](https://github.com/sripwoud/dublette/pull/5))

## [0.1.1](https://github.com/sripwoud/dublette/compare/v0.1.0...v0.1.1) - 2026-02-25

### Fixed

- _(docs)_ update main README
