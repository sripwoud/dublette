# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1](https://github.com/sripwoud/dublette/compare/v0.5.0...v0.5.1) - 2026-09-03

### Fixed

- *(deps)* update all non-major dependencies ([#52](https://github.com/sripwoud/dublette/pull/52))

### Other

- *(deps)* update rust crate which to v8 ([#58](https://github.com/sripwoud/dublette/pull/58))
- release on cargo dependency bumps ([#57](https://github.com/sripwoud/dublette/pull/57))
- refresh apt index before installing ffmpeg ([#55](https://github.com/sripwoud/dublette/pull/55))
- *(deps)* update amannn/action-semantic-pull-request action to v6 ([#53](https://github.com/sripwoud/dublette/pull/53))
- adopt renovate, pin mise tools, fix clippy 1.98 lint ([#51](https://github.com/sripwoud/dublette/pull/51))
- bump jdx/mise-action to v4 and changed-files to v47 ([#49](https://github.com/sripwoud/dublette/pull/49))
- bump actions/checkout to v7 ([#48](https://github.com/sripwoud/dublette/pull/48))
- *(release)* authenticate release-plz with a github app token ([#47](https://github.com/sripwoud/dublette/pull/47))

## [0.5.0](https://github.com/sripwoud/dublette/compare/v0.4.0...v0.5.0) - 2026-08-12

### Added

- *(cli)* add --keep-in to pin the surviving copy to a directory ([#45](https://github.com/sripwoud/dublette/pull/45))
- *(json)* report skipped files in JSON output ([#46](https://github.com/sripwoud/dublette/pull/46))
- *(cli)* add --version flag ([#41](https://github.com/sripwoud/dublette/pull/41))

### Other

- *(meta)* correct encoding-match claims and audio-stale glossary entries
- *(audio)* explain acoustic fingerprinting

## [0.4.0](https://github.com/sripwoud/dublette/compare/v0.3.0...v0.4.0) - 2026-08-12

### Added

- *(audio)* extend deduplication to audio files ([#36](https://github.com/sripwoud/dublette/pull/36))

### Fixed

- *(deps)* replace unmaintained img_hash to drop vulnerable transpose 0.1.0 ([#37](https://github.com/sripwoud/dublette/pull/37))
- *(ci)* point workflow branch filters at master ([#38](https://github.com/sripwoud/dublette/pull/38))

### Other

- *(meta)* add audio domain terms and ADR-0002 two-strategy audio matching

## [0.3.0](https://github.com/sripwoud/dublette/compare/v0.2.1...v0.3.0) - 2026-05-05

### Added

- *(dedupe)* implement plan() function ([#33](https://github.com/sripwoud/dublette/pull/33))
- *(dedupe)* wire dedupe module into lib.rs

### Other

- *(lib)* switch run() to use dedupe::plan() and slim shell ([#34](https://github.com/sripwoud/dublette/pull/34))
- *(dedupe)* migrate DuplicateGroup and HashedFile to PathBuf in dedupe module ([#32](https://github.com/sripwoud/dublette/pull/32))
- add CONTEXT, ADR-0001, and plan for dedupe core extraction
- add meta folder

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
