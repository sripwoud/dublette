# Acoustic Fingerprinting

## Why Byte-Level and Perceptual Hashing Both Fail for Audio

Checksums (MD5, SHA-256) key on raw bytes. Re-encoding the same recording -- flac to mp3, 320 kbps down to 128 kbps, one container to another -- rewrites every byte of the encoded stream. Writing a tag rewrites bytes too. Two files carrying the same music share no checksum.

Perceptual hashing does not transfer either. It hashes a visual projection: a small grayscale grid, brightness gradients between neighboring cells. Videos are hashed _as images_ (one extracted frame). Audio has no frame and no visual projection to reduce.

Audio needs a metric computed from what the file sounds like.

## What Is an Acoustic Fingerprint?

An acoustic fingerprint is a sequence of items derived from the decoded audio signal, one item per short time window. Each item encodes how energy is distributed across pitch classes in that window. The sequence describes the shape of the recording over time.

Two files of the same recording produce near-identical sequences even when they differ in:

- Container and codec (flac, mp3, m4a, opus)
- Bitrate and lossy compression artifacts
- Tags (title, artist, embedded artwork)
- Sample rate and channel count
- A few seconds of leading silence or a trimmed intro, which alignment mostly absorbs

Unlike a perceptual hash, a fingerprint is not fixed-size. Its length grows with the duration of the audio, which rules out the fixed-width comparison used for images.

## Decode and Fingerprint

Two stages per file, joined by a pipe -- ffmpeg runs as a child process, the fingerprinter runs inside dublette:

1. ffmpeg decodes the file's first audio stream (`-map 0:a:0`) to mono 16-bit signed little-endian PCM at 11025 Hz, capped at the first 120 seconds, and writes it to stdout.
2. Dublette reads that PCM off the pipe and feeds it to the pure-Rust `rusty-chromaprint` crate, configured with `Configuration::preset_test2`. The fingerprinter emits a sequence of 32-bit sub-fingerprint items, one per time window.

No temporary files, no `fpcalc`. The PCM never touches disk, and the only external tool is the ffmpeg binary already required for the video pass.

Downmixing to one channel at 11025 Hz is deliberate. The fingerprint only needs low and mid frequency structure, so channel layout and sample rate are differences the fingerprinter never sees.

Audio too short to yield a single fingerprint item -- roughly under 3 seconds -- becomes a **skipped file** with a reason, as does a file ffmpeg cannot decode. Neither is silently dropped.

## Dissimilarity Instead of Hamming Distance

Hamming distance requires two hashes of equal, fixed length. Fingerprints are variable-length, and the same recording can begin at a different offset in each file. Audio therefore uses a different metric.

`match_fingerprints` first _aligns_ the two sequences -- finding the offset at which they overlap -- then reports the matched segments with a per-segment error `score`. Dublette normalizes that result into a dissimilarity between `0.0` and `1.0`:

```text
total         = max(len_a, len_b)
matched_error = sum over segments of (items_count * score / 32)
unmatched     = total - sum of segment items_count
dissimilarity = (matched_error + unmatched) / total
```

Two terms, two ways to disagree:

- `matched_error` -- how badly the aligned items differ, bit by bit. This is where lossy compression artifacts land.
- `unmatched` -- how much of the longer fingerprint found no counterpart at all. This is where differing duration lands.

A dissimilarity of `0.0` means every item aligned with no error. `1.0` means nothing aligned: when the matcher finds no overlap it reports no segments, so the entire length counts as `unmatched`.

Alignment keeps a shifted start from corrupting the per-item error, but the non-overlapping head or tail still counts as `unmatched`. Each item covers about 0.124 seconds, so 120 seconds is roughly 970 items and every second of offset costs about `0.01` of dissimilarity. The default threshold of `0.1` therefore tolerates around 9 seconds of added silence or trimmed intro, not an arbitrary shift.

Squashing each fingerprint into a fixed-size hash so that audio could reuse the image comparison path was considered and rejected for exactly this reason -- it trades alignment robustness for code shape.

## The Audio Threshold

`--audio-threshold` sets the maximum dissimilarity at which two audio files join a **duplicate group**. The comparison is `<=`. It applies to recording match only; combining it with `--audio-match encoding` is an error.

| Threshold       | Behavior                                                                                            |
| --------------- | --------------------------------------------------------------------------------------------------- |
| `0.0`           | Every aligned item must agree exactly and cover the full length. Rejects most lossy re-encodes.     |
| `0.05`          | Strict. Matches the same recording across comparable encodes; a low-bitrate rip may fall outside.   |
| `0.1` (default) | Matches the same recording across codecs and bitrates while rejecting different recordings.         |
| `0.2`           | Lenient. Absorbs heavy compression artifacts and some unmatched length. Higher false-positive risk. |
| `0.3+`          | Aggressive. Distinct recordings that share key, tempo, and instrumentation may be grouped.          |

Start with the default of `0.1` and use `--dry-run` to verify results before loosening.

Threshold is a per-kind concept: `--threshold` counts hamming bits for image and video, `--audio-threshold` is a normalized ratio for audio. The two numbers are not comparable.

## Choosing What to Keep

Recording match deliberately groups files of unequal fidelity, so alphabetical order alone could keep a 128 kbps rip and delete its flac source. Recording-match groups use a quality-aware **keep policy** instead: lossless (flac, wav, aiff) first, then higher bitrate, then alphabetical tiebreak. Fidelity metadata is read with `lofty`; a file whose container cannot be parsed ranks bottom. Every other kind of group, including encoding-match audio, keeps the alphabetically first file.

## Encoding Match, the Strict Alternative

`--audio-match encoding` answers a narrower question: is this the same encoded audio, retagged?

Each file is copied to a temporary location, every tag region `lofty` recognizes is removed, and the remaining bytes -- audio payload plus container structure -- are hashed to a 64-bit digest. Files group on digest equality; there is no byte-for-byte re-check.

|             | Recording match              | Encoding match                     |
| ----------- | ---------------------------- | ---------------------------------- |
| Question    | Same recording?              | Same encoded stream, retagged?     |
| Input       | Decoded PCM, first 120s      | File bytes minus tag regions       |
| ffmpeg      | Required                     | Not used                           |
| Metric      | Dissimilarity `<=` threshold | 64-bit digest equality             |
| Comparison  | Pairwise, O(n^2)             | Bucket by digest, no pairwise pass |
| Keep policy | Highest fidelity             | Alphabetically first               |

For files both strategies can process, encoding match finds a subset of the pairs recording match finds. A retagged copy matches. A re-encode does not, and neither does a remux of the same stream into a different container -- nor a lossless flac to wav transcode, which is deliberately a recording match and not an encoding match.

The strategies do not nest in general: recording match skips audio under roughly 3 seconds and anything ffmpeg cannot decode, while encoding match still groups retagged copies of those files.

Pick encoding match when deletion safety outweighs recall (it can never group a lossless file with a lossy re-encode), when ffmpeg is unavailable, or when the collection is large enough that decoding it is not worth the time. One gap: wma files have no tag parser support, so encoding match reports them as skipped files, while recording match reads them through ffmpeg.

If ffmpeg is missing under recording match, the audio pass is skipped with a warning and encoding match still works. Dublette never falls back silently: switching strategy mid-run would change what "duplicate" means for that run.

Audio streams embedded in video files are never fingerprinted under either strategy. Duplicate groups are kind-pure -- a music video and its audio rip are not duplicates of each other.

## Limitations

- **Partial overlap is not a match.** A 30-second clip compared against the 5-minute track it was cut from aligns only where the two overlap; the rest of the longer fingerprint stays unmatched, so dissimilarity stays high. The question answered is "same recording", not "contained in".
- **Only the first 120 seconds are compared.** Files that are identical up to the 2-minute mark and diverge after it look identical. A download truncated past that mark can be grouped with the complete file.
- **A different performance is a different recording.** A live version or a cover will not match the studio original, and a remaster may not either. Matching is recording-level, not song-level. Conversely, an edit that only alters the audio past the 2-minute mark still matches, because the compared window never reaches the difference.
- **Near-silent or very short audio fingerprints poorly.** Few items and little structure make the result unreliable at any threshold; under roughly 3 seconds there is nothing to fingerprint and the file is skipped. Encoding match still handles those files.
- **Broad harmonic similarity can collide at loose thresholds.** Distinct recordings sharing instrumentation, key, and tempo may fall inside a widened threshold.

Always use `--dry-run` to review results before deleting files.
