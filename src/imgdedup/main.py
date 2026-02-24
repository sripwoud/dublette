import argparse
import shutil
import subprocess
import sys
import tempfile
from collections.abc import Mapping
from pathlib import Path

import imagehash
from PIL import Image

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
MEDIA_EXTENSIONS = IMAGE_EXTENSIONS | VIDEO_EXTENSIONS


def _get_ffmpeg() -> str:
    path = shutil.which("ffmpeg")
    if not path:
        raise RuntimeError("ffmpeg not found on PATH")
    return path


def _run_ffmpeg_extract(
    ffmpeg: str, video_path: str, seek: str, output: str
) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [ffmpeg, "-y", "-ss", seek, "-i", video_path, "-frames:v", "1", output],
        capture_output=True,
        timeout=30,
    )


def _extract_frame_hash(video_path: Path, ffmpeg: str) -> imagehash.ImageHash:
    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        for seek in ("1", "0"):
            result = _run_ffmpeg_extract(ffmpeg, str(video_path), seek, tmp_path)
            if result.returncode != 0:
                continue
            if Path(tmp_path).stat().st_size > 0:
                img = Image.open(tmp_path)
                return imagehash.phash(img)

        stderr = result.stderr.decode(errors="replace").strip()
        raise RuntimeError(f"ffmpeg could not extract frame: {stderr}")
    finally:
        Path(tmp_path).unlink(missing_ok=True)


def _collect_files(directory: Path, extensions: set[str]) -> list[Path]:
    return sorted(
        f
        for f in directory.rglob("*")
        if f.is_file() and f.suffix.lower() in extensions and f.stat().st_size > 0
    )


def _relative_key(file: Path, directory: Path) -> str:
    return str(file.relative_to(directory))


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


def delete_empty_files(directory: Path, dry: bool) -> int:
    pattern = directory.rglob("*")
    empty = sorted(
        f
        for f in pattern
        if f.is_file()
        and f.suffix.lower() in MEDIA_EXTENSIONS
        and f.stat().st_size == 0
    )
    if not empty:
        return 0

    print(f"Found {len(empty)} empty (0-byte) file(s):\n")
    for f in empty:
        print(f"    {f.relative_to(directory)}")

    if dry:
        print()
        return len(empty)

    print()
    for f in empty:
        f.unlink()
        print(f"  Deleted: {f}")

    return len(empty)


def find_image_duplicates(directory: Path, threshold: int) -> dict[str, list[str]]:
    image_files = _collect_files(directory, IMAGE_EXTENSIONS)

    hashes: dict[str, imagehash.ImageHash] = {}
    for f in image_files:
        try:
            img = Image.open(f)
            hashes[_relative_key(f, directory)] = imagehash.phash(img)
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

    return duplicates


def find_video_duplicates(directory: Path, threshold: int) -> dict[str, list[str]]:
    ffmpeg = _get_ffmpeg()
    video_files = _collect_files(directory, VIDEO_EXTENSIONS)

    hashes: dict[str, imagehash.ImageHash] = {}
    for f in video_files:
        try:
            hashes[_relative_key(f, directory)] = _extract_frame_hash(f, ffmpeg)
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
    threshold: int,
    dry: bool,
    only: str | None,
    delete_empty: bool,
) -> None:
    if not directory.is_dir():
        print(f"Error: {directory} is not a directory", file=sys.stderr)
        sys.exit(1)

    total_deleted = 0

    if delete_empty:
        total_deleted += delete_empty_files(directory, dry)

    if only in (None, "images"):
        image_files = _collect_files(directory, IMAGE_EXTENSIONS)
        if not image_files:
            print("No images found.")
        else:
            print(f"Scanning {len(image_files)} image(s)...")
            duplicates = find_image_duplicates(directory, threshold)
            groups = build_duplicate_groups(duplicates)
            total_deleted += report_and_delete("image", groups, directory, dry)

    if only in (None, "videos"):
        video_files = _collect_files(directory, VIDEO_EXTENSIONS)
        if not video_files:
            print("No videos found.")
        else:
            print(f"Scanning {len(video_files)} video(s)...")
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
        "-t",
        "--threshold",
        type=int,
        default=9,
        help="Max hamming distance to consider as duplicate (default: 9)",
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
    parser.add_argument(
        "--delete-empty",
        action="store_true",
        help="Delete 0-byte media files",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    try:
        run(
            args.directory,
            args.threshold,
            args.dry_run,
            args.only,
            args.delete_empty,
        )
    except KeyboardInterrupt:
        print("\nInterrupted.", file=sys.stderr)
        sys.exit(130)


if __name__ == "__main__":
    main()
