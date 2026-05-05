# Plan: Carve a `dedupe` core out of `lib.rs::run`

## Context

This plan implements **Candidate 1** from a `/improve-codebase-architecture` session run on 2026-05-05.

Today, `lib.rs::run(&Args) -> Result<bool>` is the only callable seam in the deduplication pipeline. Its observable behaviour leaks across stdout, stderr, the filesystem (deletions), and an interactive prompt; the only escaping value is "did we find any duplicates?". As a result, every behaviour test in `tests/cli.rs` spawns the binary and greps stdout. The interface is too coarse to be a useful test surface.

The fix: extract a deep `dedupe` module that returns a `DeduplicationReport` value. The shell (today's `run`) becomes a thin orchestrator that renders, prompts, and deletes.

## Decisions locked during grilling

| #  | Decision                                                                                                                                                                                                                                               | Reasoning                                                                                                                                              |
| -- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Q1 | **Compute-only core** (option C). Core returns a report value; shell does printing, prompting, and deletion. No `plan/apply` split, no `Confirmer` trait.                                                                                              | Smallest cut that fixes the test-surface problem. Confirmer would be a hypothetical seam (one CLI adapter only) — `LANGUAGE.md` flags this as a smell. |
| Q2 | **Report shape**: tagged flat `Vec<DuplicateGroup>` with a `kind: MediaKind` field; skipped files become data on the report; `to_delete` precomputed and includes empties when configured; `PathBuf` everywhere (no more `String` for relative paths). | Single source of truth. Shell stays domain-stupid: just iterate `to_delete`.                                                                           |
| Q3 | **`Progress` trait sink** (option B). CLI passes an `IndicatifProgress` adapter; tests pass `NoopProgress`. Verbose output streams via `progress.diag(...)`.                                                                                           | Two real adapters today (CLI + test no-op + `--quiet`/`--json` no-op) — passes the _two-adapter_ seam test. Indicatif stays out of the core.           |
| Q4 | **Layout A**: flat `src/dedupe.rs`, module name `dedupe`.                                                                                                                                                                                              | One new file; preserves the existing flat layout. Folder module is reserved for if `dedupe.rs` outgrows comfort.                                       |

## A. File layout

```
src/
├── main.rs       unchanged
├── lib.rs        module decls + run() shell (slimmed down)
├── cli.rs        unchanged
├── dedupe.rs     NEW
├── scan.rs       loses DuplicateGroup/HashedFile; keeps walk + grouping internals
├── hash.rs       unchanged
├── report.rs     format_table/format_json retargeted at DeduplicationReport
└── delete.rs     unchanged
```

## B. Module surface (`src/dedupe.rs`)

```rust
// Public types
pub struct Config {
    pub threshold: u32,
    pub only: Option<MediaKind>,
    pub include_empty: bool,
}

pub enum MediaKind { Image, Video }

pub struct DeduplicationReport {
    pub groups: Vec<DuplicateGroup>,
    pub empty_files: Vec<PathBuf>,
    pub skipped: Vec<SkippedFile>,
    pub to_delete: Vec<PathBuf>,
}

pub struct DuplicateGroup {
    pub kind: MediaKind,
    pub keep: PathBuf,
    pub duplicates: Vec<PathBuf>,
}

pub struct SkippedFile {
    pub path: PathBuf,
    pub reason: String,
}

// Public trait + adapters
pub trait Progress {
    fn phase_start(&self, label: &str, total: u64);
    fn tick(&self);
    fn phase_finish(&self);
    fn diag(&self, msg: &str);
}

pub struct NoopProgress;        // for tests, --quiet, --json
pub struct IndicatifProgress { /* verbose flag + indicatif state */ }

// Public entry point
pub fn plan(
    dirs: &[PathBuf],
    config: &Config,
    progress: &dyn Progress,
) -> eyre::Result<DeduplicationReport>;

// Internal
struct HashedFile { path: PathBuf, hash: ImageHash }   // moved from scan.rs
```

## C. `plan()` pseudocode

- Walk: `delete::find_empty_files` only when `config.include_empty`; collect images + videos via `scan::collect_files` per kind, gated by `config.only`.
- Hash: parallel via rayon. Successes → `HashedFile`. Failures → `SkippedFile { path, reason: format!("{e}") }`. Each iteration: `progress.tick()`. Verbose-relevant data: `progress.diag(...)` (unconditional from core; adapter decides whether to print).
- ffmpeg: if `find_ffmpeg()` fails, video pass is silently skipped (today's behaviour). No `SkippedFile` for "ffmpeg not found" because no specific file failed.
- Group: per-kind, build adjacency via `scan::pairwise_compare`, then connected components via `scan::build_duplicate_groups`; tag each resulting group with its `MediaKind`. (Both functions remain in `scan.rs` for now; candidate 2 collapses them later.)
- Compose: `to_delete = groups.flat_map(|g| g.duplicates).chain(empty_files iff include_empty).collect()`.

`verbose` does **not** appear in `Config`. The core unconditionally calls `progress.diag(...)`; the indicatif adapter takes `verbose: bool` at construction and silently no-ops `diag()` when off. Presentation concerns stay out of the algorithm.

## D. TDD plan

**Unit tests in `dedupe.rs`** (call `plan()` with tempdirs, assert on returned report):

- `plan_empty_dirs_returns_empty_report`
- `plan_identical_images_yield_one_group_kind_image`
- `plan_threshold_zero_excludes_near_matches`
- `plan_threshold_loose_includes_near_matches`
- `plan_only_images_skips_videos`
- `plan_only_videos_skips_images`
- `plan_include_empty_populates_empty_files_and_to_delete`
- `plan_exclude_empty_omits_empty_files`
- `plan_unreadable_image_recorded_as_skipped`
- `plan_to_delete_uses_pathbuf_not_string`
- `plan_multiple_directories_merge_into_one_pipeline`
- `noop_progress_does_nothing` (smoke)
- `recording_progress_captures_phase_lifecycle` (in-test recorder validates `phase_start` / `tick` / `phase_finish` ordering)

**Existing unit tests** stay green after migration:

- `scan.rs` tests update to `PathBuf`-typed adjacency / groups.
- `report.rs` tests update to consume `DeduplicationReport`.

**Existing integration tests** in `tests/cli.rs` stay green **without modification** — that's the proof that the shell refactor is behaviour-preserving.

**Run command**: `cargo test --all` (or via `mise run test` if defined).

## E. Commit list

| # | Commit                                                          | Phase           | Layers | Changes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Tests                                                                                                            |
| - | --------------------------------------------------------------- | --------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| 1 | `feat(dedupe): introduce module with progress trait and config` | Scaffold        | core   | New `src/dedupe.rs` with `Config`, `MediaKind`, `SkippedFile`, `Progress`, `NoopProgress`, `IndicatifProgress`. `lib.rs` adds `pub mod dedupe;`.                                                                                                                                                                                                                                                                                                                                                                                     | `noop_progress_does_nothing`, `recording_progress_captures_phase_lifecycle`                                      |
| 2 | `refactor(dedupe): migrate domain types from scan to dedupe`    | Type migration  | core   | Move `DuplicateGroup` + `HashedFile` from `scan.rs` to `dedupe.rs`. Add `kind: MediaKind` to group. Switch `String` → `PathBuf` for `keep` / `duplicates` / `HashedFile.path`. Update `scan::pairwise_compare`, `scan::build_duplicate_groups`, `lib.rs::compare_hashes`, `report.rs`, all tests to new types. Add `DeduplicationReport` struct.                                                                                                                                                                                     | All existing `scan.rs` / `report.rs` tests updated to `PathBuf` and continue passing.                            |
| 3 | `feat(dedupe): implement plan() function`                       | Core behaviour  | core   | Implement `plan(dirs, config, progress) -> Report`. Internally calls `scan::collect_files`, `hash::*`, `scan::pairwise_compare`, `scan::build_duplicate_groups`, `delete::find_empty_files`. Captures skipped files; computes `to_delete`. No callers yet.                                                                                                                                                                                                                                                                           | All 11 `plan_*` tests above.                                                                                     |
| 4 | `refactor(lib): switch run() to use plan() and slim shell`      | Shell migration | shell  | `run()` becomes thin shell: build `Config` from `Args`, build `IndicatifProgress` (or `NoopProgress` for `--quiet`/`--json`), call `plan()`, render via `report::format_table`/`format_json`, prompt, delete. Retarget `format_table`/`format_json` at `&DeduplicationReport`. Delete `lib.rs::compare_hashes`, `process_media`, `hash_images`, `hash_videos`, `make_progress_bar`. Delete `report::JsonReport` / `JsonGroup` / `resolve_deletions`. Skipped files rendered post-plan via `eprintln!` (preserves today's behaviour). | Existing `tests/cli.rs` integration tests pass **unchanged** — proof the shell refactor is behaviour-preserving. |
| 5 | `chore: post-refactor PR review and comment cleanup`            | Cleanup         | all    | Diff vs `master`, self-review as outside engineer, address findings. Remove unnecessary comments per planning rule F. Final `cargo test --all`.                                                                                                                                                                                                                                                                                                                                                                                      | Existing tests still green.                                                                                      |

## F. PR review todo

- After commit 4, do a self-review of the branch as if it were someone else's PR. Note findings.
- Apply recommendations; iterate tests until green.
- Sweep for unnecessary comments; keep only Rust doc comments where they add genuine value.

## Resolved sub-decisions

- **Skipped files rendering**: option (c) — preserve today's behaviour, but driven from the shell post-plan by iterating `report.skipped` and `eprintln!`-ing each one. Smallest user-visible change.

## Out of scope (follow-up candidates)

- **Candidate 2** — fuse `compare_hashes` / `pairwise_compare` / `build_duplicate_groups` into a single deep similarity-grouping function inside `dedupe`.
- **Candidate 3** — already folded into this plan via the small `Config` type replacing `&Args` in the core.
- **Candidate 4** — unify the directory walks into a single intake pass.
- **Candidate 5** (small) — likely subsumed by commit 4 (which deletes `report::resolve_deletions`).
