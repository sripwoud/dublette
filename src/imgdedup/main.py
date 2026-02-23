import argparse
import sys
from collections.abc import Mapping
from pathlib import Path

from imagededup.methods import AHash, DHash, PHash, WHash

HASHERS = {
    "phash": PHash,
    "ahash": AHash,
    "dhash": DHash,
    "whash": WHash,
}

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".bmp", ".gif", ".tiff", ".webp"}


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


def run(directory: Path, method: str, threshold: int, dry: bool) -> None:
    if not directory.is_dir():
        print(f"Error: {directory} is not a directory", file=sys.stderr)
        sys.exit(1)

    image_count = sum(
        1 for f in directory.iterdir() if f.suffix.lower() in IMAGE_EXTENSIONS
    )
    if image_count == 0:
        print(f"No images found in {directory}")
        return

    hasher = HASHERS[method]()
    duplicates = hasher.find_duplicates(
        image_dir=str(directory), max_distance_threshold=threshold
    )

    groups = build_duplicate_groups(duplicates)
    if not groups:
        print("No duplicates found.")
        return

    to_delete = resolve_deletions(groups, directory)

    print(
        f"Found {len(groups)} duplicate group(s), {len(to_delete)} file(s) to remove:\n"
    )
    for i, group in enumerate(groups, 1):
        sorted_files = sorted(group)
        print(f"  Group {i}:")
        print(f"    keep:   {sorted_files[0]}")
        for f in sorted_files[1:]:
            print(f"    delete: {f}")
        print()

    if dry:
        print("[dry run] No files were deleted.")
        return

    for path in to_delete:
        path.unlink()
        print(f"  Deleted: {path}")

    print(f"\nRemoved {len(to_delete)} duplicate(s).")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="imgdedup",
        description="Deduplicate images using perceptual hashing.",
    )
    parser.add_argument(
        "directory", type=Path, help="Folder containing images to deduplicate"
    )
    parser.add_argument(
        "-m",
        "--method",
        choices=HASHERS.keys(),
        default="phash",
        help="Hashing method (default: phash)",
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
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    run(args.directory, args.method, args.threshold, args.dry_run)


if __name__ == "__main__":
    main()
