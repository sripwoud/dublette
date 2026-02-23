import argparse
import shutil
import subprocess
import sys
from collections.abc import Mapping
from pathlib import Path

from imagededup.methods import AHash, DHash, PHash, WHash

import videohash2.videoduration as _vd
import videohash2.videohash as _vh


def _video_duration_fixed(path: str | None = None, **_kwargs: object) -> float:
    ffprobe_path = shutil.which("ffprobe")
    if not ffprobe_path:
        raise RuntimeError("ffprobe not found on PATH")
    result = subprocess.run(
        [
            ffprobe_path,
            "-v",
            "quiet",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            path,
        ],
        capture_output=True,
        text=True,
    )
    output = result.stdout.strip()
    if not output:
        raise RuntimeError(f"ffprobe could not determine duration for {path}")
    return float(output)


_vd.video_duration = _video_duration_fixed
_vh.video_duration = _video_duration_fixed

from videohash2 import VideoHash  # noqa: E402

HASHERS = {
    "phash": PHash,
    "ahash": AHash,
    "dhash": DHash,
    "whash": WHash,
}

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".bmp", ".gif", ".tiff", ".webp"}
VIDEO_EXTENSIONS = {
    ".mp4",
    ".mov",
    ".avi",
    ".mkv",
    ".wmv",
    ".flv",
    ".webm",
    ".m4v",
    ".3gp",
}


def build_duplicate_groups(duplicates: Mapping[str, list[str]]) -> list[set[str]]:
    visited: set[str] = set()
    groups: list[set[str]] = []

    for filename, dupes in duplicates.items():
        if filename in visited or not dupes:
            continue

        group: set[str] = {filename}
        stack = list(dupes)
        while stack:
            current = stack.pop()
            if current in group:
                continue
            group.add(current)
            for neighbor in duplicates.get(current, []):
                if neighbor not in group:
                    stack.append(neighbor)

        visited.update(group)
        groups.append(group)

    return groups


def resolve_deletions(groups: list[set[str]], directory: Path) -> list[Path]:
    to_delete: list[Path] = []

    for group in groups:
        sorted_files = sorted(group)
        to_delete.extend(directory / f for f in sorted_files[1:])

    return to_delete


def find_video_duplicates(directory: Path, threshold: int) -> dict[str, list[str]]:
    video_files = sorted(
        f for f in directory.iterdir() if f.suffix.lower() in VIDEO_EXTENSIONS
    )

    hashes: dict[str, VideoHash] = {}
    for f in video_files:
        try:
            hashes[f.name] = VideoHash(path=str(f))
        except Exception as e:
            print(f"  Warning: skipping {f.name}: {e}", file=sys.stderr)

    duplicates: dict[str, list[str]] = {name: [] for name in hashes}
    names = list(hashes.keys())

    for i in range(len(names)):
        for j in range(i + 1, len(names)):
            distance = hashes[names[i]] - hashes[names[j]]
            if distance <= threshold:
                duplicates[names[i]].append(names[j])
                duplicates[names[j]].append(names[i])

    for vh in hashes.values():
        vh.delete_storage_path()

    return duplicates


def report_and_delete(
    label: str,
    groups: list[set[str]],
    directory: Path,
    dry: bool,
) -> int:
    if not groups:
        print(f"No duplicate {label} found.")
        return 0

    to_delete = resolve_deletions(groups, directory)

    print(
        f"Found {len(groups)} duplicate {label} group(s), {len(to_delete)} file(s) to remove:\n"
    )
    for i, group in enumerate(groups, 1):
        sorted_files = sorted(group)
        print(f"  Group {i}:")
        print(f"    keep:   {sorted_files[0]}")
        for f in sorted_files[1:]:
            print(f"    delete: {f}")
        print()

    if dry:
        return len(to_delete)

    for path in to_delete:
        path.unlink()
        print(f"  Deleted: {path}")

    return len(to_delete)


def run(
    directory: Path,
    method: str,
    threshold: int,
    dry: bool,
    only: str,
) -> None:
    if not directory.is_dir():
        print(f"Error: {directory} is not a directory", file=sys.stderr)
        sys.exit(1)

    total_deleted = 0

    if only in (None, "images"):
        image_count = sum(
            1 for f in directory.iterdir() if f.suffix.lower() in IMAGE_EXTENSIONS
        )
        if image_count == 0:
            print("No images found.")
        else:
            print(f"Scanning {image_count} image(s)...")
            hasher = HASHERS[method]()
            duplicates = hasher.find_duplicates(
                image_dir=str(directory), max_distance_threshold=threshold
            )
            groups = build_duplicate_groups(duplicates)
            total_deleted += report_and_delete("image", groups, directory, dry)

    if only in (None, "videos"):
        video_count = sum(
            1 for f in directory.iterdir() if f.suffix.lower() in VIDEO_EXTENSIONS
        )
        if video_count == 0:
            print("No videos found.")
        else:
            print(f"Scanning {video_count} video(s)...")
            duplicates = find_video_duplicates(directory, threshold)
            groups = build_duplicate_groups(duplicates)
            total_deleted += report_and_delete("video", groups, directory, dry)

    if dry:
        print(f"\n[dry run] {total_deleted} file(s) would be deleted.")
    elif total_deleted:
        print(f"\nRemoved {total_deleted} duplicate(s) total.")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="imgdedup",
        description="Deduplicate images and videos using perceptual hashing.",
    )
    parser.add_argument(
        "directory", type=Path, help="Folder containing media to deduplicate"
    )
    parser.add_argument(
        "-m",
        "--method",
        choices=HASHERS.keys(),
        default="phash",
        help="Image hashing method (default: phash)",
    )
    parser.add_argument(
        "-t",
        "--threshold",
        type=int,
        default=10,
        help="Max hamming distance to consider as duplicate (default: 10)",
    )
    parser.add_argument(
        "-n",
        "--dry-run",
        action="store_true",
        help="List duplicates without deleting",
    )
    parser.add_argument(
        "--only",
        choices=("images", "videos"),
        default=None,
        help="Process only images or only videos (omit to process all)",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    try:
        run(args.directory, args.method, args.threshold, args.dry_run, args.only)
    except KeyboardInterrupt:
        print("\nInterrupted.", file=sys.stderr)
        sys.exit(130)


if __name__ == "__main__":
    main()
