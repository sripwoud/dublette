# Quick Start

This guide walks through a typical first use of dublette.

## 1. Preview Duplicates (Dry Run)

Start with a dry run to see what dublette would do without deleting anything:

```bash
dublette ~/Photos --dry-run
```

Dublette scans the directory recursively, hashes each image and video, compares them, and prints a table of duplicate groups:

```
Scanning 142 image(s)...
Hashing images [========================================] 142/142 (0s)
Comparing images [========================================] 10011/10011 (0s)
Duplicate images: 3 group(s), 5 to remove
+-------+----------------------------+---------------+
| Group | File                       | Action        |
+-------+----------------------------+---------------+
| 1     | vacation/beach.jpg         | keep          |
|       | vacation/beach_copy.jpg    | would delete  |
|       | backup/beach.jpg           | would delete  |
+-------+----------------------------+---------------+
| 2     | portraits/alice.png        | keep          |
|       | portraits/alice (1).png    | would delete  |
+-------+----------------------------+---------------+
| 3     | misc/sunset.jpg            | keep          |
|       | misc/sunset_edited.jpg     | would delete  |
+-------+----------------------------+---------------+

[dry run] 5 file(s) would be deleted.
```

Within each group, dublette keeps the alphabetically first file and marks the rest for deletion.

## 2. Understand the Output

- **Group** -- a set of files that are perceptually similar (within the hamming distance threshold)
- **keep** -- the file that will be preserved (alphabetically first in the group)
- **would delete** / **delete** -- the files that are duplicates and will be removed

In dry-run mode, no files are modified. The exit code is `1` if duplicates were found, `0` if none.

## 3. Delete Duplicates

When you are satisfied with the preview, run without `--dry-run`:

```bash
dublette ~/Photos
```

Dublette will show the same table, then prompt for confirmation:

```
Delete 5 duplicate file(s)? [y/N]
```

Type `y` to proceed. To skip the prompt, use the `-y` flag:

```bash
dublette ~/Photos -y
```

## 4. Process Only Images or Videos

To limit processing to one media type:

```bash
dublette ~/Photos --only images
dublette ~/Videos --only videos
```

## 5. Adjust Sensitivity

The `--threshold` flag controls how similar two files must be to count as duplicates. Lower values are stricter:

```bash
dublette ~/Photos --threshold 0 --dry-run
```

A threshold of `0` requires an exact perceptual hash match. The default of `1` allows a single bit of difference, catching near-identical files that differ slightly due to re-encoding or compression.

See [Perceptual Hashing](../how-it-works/perceptual-hashing.md) for more on how the threshold works.

## Next Steps

- [CLI Reference](../cli-reference.md) -- full list of options and flags
- [Examples](../examples.md) -- practical usage scenarios
- [How It Works](../how-it-works/perceptual-hashing.md) -- understand the algorithm
