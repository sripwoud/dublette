# Examples

## Clean Up a Photo Library

Preview duplicates in your photos directory:

```bash
dublette ~/Photos --dry-run
```

Review the output, then delete:

```bash
dublette ~/Photos --yes
```

To also remove any 0-byte corrupted files:

```bash
dublette ~/Photos --delete-empty --yes
```

## Deduplicate a Video Archive

Process only video files:

```bash
dublette ~/Videos --only videos --dry-run
```

Videos are compared by extracting a frame at the 1-second mark and hashing it. This catches re-encoded copies and different containers of the same content.

## Images Only (Skip Video Processing)

If you do not have ffmpeg installed, or want to avoid the overhead of frame extraction:

```bash
dublette ~/Media --only images
```

## Adjust the Threshold

The default threshold of `1` is conservative. To require exact perceptual matches:

```bash
dublette ~/Photos --threshold 0 --dry-run
```

To catch more aggressively similar files (e.g., same photo with different crops or slight edits):

```bash
dublette ~/Photos --threshold 3 --dry-run
```

Always preview with `--dry-run` when increasing the threshold.

## Scripting with JSON Output

Use `--json` and `--dry-run` to get machine-readable output:

```bash
dublette ~/Photos --dry-run --json > results.json
```

The exit code indicates whether duplicates were found:

```bash
dublette ~/Photos --dry-run --json --quiet
if [ $? -eq 1 ]; then
  echo "Duplicates found"
fi
```

Parse the JSON output with `jq`:

```bash
dublette ~/Photos --dry-run --json | jq '.groups[].duplicates[]'
```

Count total duplicates:

```bash
dublette ~/Photos --dry-run --json | jq '.total_duplicates'
```

## CI / Automated Pipeline

Combine `--json`, `--quiet`, and `--yes` for non-interactive use:

```bash
dublette /data/media --json --quiet --yes > /tmp/dedup-report.json
```

Use the exit code in a CI step to detect duplicates without deleting:

```bash
dublette /data/uploads --dry-run --quiet
EXIT_CODE=$?
if [ "$EXIT_CODE" -eq 1 ]; then
  echo "::warning::Duplicate media files detected"
fi
```

## Verbose Output for Debugging

To see the computed hash for each file and the hamming distance for every comparison:

```bash
dublette ~/Photos --verbose --dry-run 2> hashes.log
```

Verbose output is written to stderr, so it can be captured separately from the table or JSON output on stdout.

## Suppress Progress Bars

When piping output or running in a non-interactive terminal:

```bash
dublette ~/Photos --quiet --dry-run
```

This suppresses the progress bars and scanning status messages while still printing the results table.

## Combine Flags

Delete all duplicates and empty files without prompts, with JSON output, no progress bars:

```bash
dublette ~/Photos --delete-empty --yes --json --quiet
```

Preview image-only duplicates with strict matching and verbose logging:

```bash
dublette ~/Photos --only images --threshold 0 --dry-run --verbose
```
