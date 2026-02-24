import shutil
import subprocess
import sys
import tempfile
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path

import click
import imagehash
from PIL import Image
from rich.console import Console
from rich.progress import Progress
from rich.table import Table

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


@dataclass
class UI:
    console: Console = field(default_factory=lambda: Console(stderr=True))
    output: Console = field(default_factory=lambda: Console())
    quiet: bool = False
    verbose: bool = False


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


def delete_empty_files(directory: Path, dry: bool, yes: bool, ui: UI) -> int:
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

    table = Table(title=f"Empty (0-byte) files ({len(empty)})")
    table.add_column("File", style="red")
    table.add_column("Action", style="dim" if dry else "red")
    for f in empty:
        action = "would delete" if dry else "delete"
        table.add_row(str(f.relative_to(directory)), action)
    ui.output.print(table)

    if dry:
        return len(empty)

    if not yes:
        click.confirm(f"Delete {len(empty)} empty file(s)?", abort=True)

    for f in empty:
        f.unlink()
        ui.console.print(f"  [red]Deleted:[/red] {f.relative_to(directory)}")

    return len(empty)


def find_image_duplicates(
    directory: Path, threshold: int, ui: UI
) -> dict[str, list[str]]:
    image_files = _collect_files(directory, IMAGE_EXTENSIONS)

    hashes: dict[str, imagehash.ImageHash] = {}
    with Progress(console=ui.console, disable=ui.quiet, transient=True) as progress:
        task = progress.add_task("Hashing images", total=len(image_files))
        for f in image_files:
            try:
                img = Image.open(f)
                hashes[_relative_key(f, directory)] = imagehash.phash(img)
            except Exception as e:
                ui.console.print(f"  Warning: skipping {f.name}: {e}")
            progress.advance(task)

    duplicates: dict[str, list[str]] = {name: [] for name in hashes}
    names = list(hashes.keys())
    total_pairs = len(names) * (len(names) - 1) // 2

    with Progress(console=ui.console, disable=ui.quiet, transient=True) as progress:
        task = progress.add_task("Comparing images", total=total_pairs)
        for i in range(len(names)):
            for j in range(i + 1, len(names)):
                distance = hashes[names[i]] - hashes[names[j]]
                if distance <= threshold:
                    duplicates[names[i]].append(names[j])
                    duplicates[names[j]].append(names[i])
                progress.advance(task)

    return duplicates


def find_video_duplicates(
    directory: Path, threshold: int, ui: UI
) -> dict[str, list[str]]:
    ffmpeg = _get_ffmpeg()
    video_files = _collect_files(directory, VIDEO_EXTENSIONS)

    hashes: dict[str, imagehash.ImageHash] = {}
    with Progress(console=ui.console, disable=ui.quiet, transient=True) as progress:
        task = progress.add_task("Hashing videos", total=len(video_files))
        for f in video_files:
            try:
                hashes[_relative_key(f, directory)] = _extract_frame_hash(f, ffmpeg)
            except Exception as e:
                ui.console.print(f"  Warning: skipping {f.name}: {e}")
            progress.advance(task)

    duplicates: dict[str, list[str]] = {name: [] for name in hashes}
    names = list(hashes.keys())
    total_pairs = len(names) * (len(names) - 1) // 2

    with Progress(console=ui.console, disable=ui.quiet, transient=True) as progress:
        task = progress.add_task("Comparing videos", total=total_pairs)
        for i in range(len(names)):
            for j in range(i + 1, len(names)):
                distance = hashes[names[i]] - hashes[names[j]]
                if distance <= threshold:
                    duplicates[names[i]].append(names[j])
                    duplicates[names[j]].append(names[i])
                progress.advance(task)

    return duplicates


def report_and_delete(
    label: str,
    groups: list[set[str]],
    directory: Path,
    dry: bool,
    yes: bool,
    ui: UI,
) -> int:
    if not groups:
        ui.output.print(f"No duplicate {label} found.")
        return 0

    to_delete = resolve_deletions(groups, directory)

    table = Table(
        title=f"Duplicate {label}s: {len(groups)} group(s), {len(to_delete)} to remove"
    )
    table.add_column("Group", style="bold")
    table.add_column("File")
    table.add_column("Action")
    for i, group in enumerate(groups, 1):
        sorted_files = sorted(group)
        table.add_row(str(i), sorted_files[0], "[green]keep[/green]")
        for f in sorted_files[1:]:
            action = "[dim]would delete[/dim]" if dry else "[red]delete[/red]"
            table.add_row("", f, action)
    ui.output.print(table)

    if dry:
        return len(to_delete)

    if not yes:
        click.confirm(f"Delete {len(to_delete)} {label} file(s)?", abort=True)

    for path in to_delete:
        path.unlink()
        ui.console.print(f"  [red]Deleted:[/red] {path.relative_to(directory)}")

    return len(to_delete)


def run(
    directory: Path,
    threshold: int,
    dry: bool,
    only: str | None,
    delete_empty: bool,
    yes: bool,
    ui: UI,
) -> None:
    total_deleted = 0

    if delete_empty:
        total_deleted += delete_empty_files(directory, dry, yes, ui)

    if only in (None, "images"):
        image_files = _collect_files(directory, IMAGE_EXTENSIONS)
        if not image_files:
            ui.output.print("No images found.")
        else:
            ui.console.print(f"Scanning {len(image_files)} image(s)...")
            duplicates = find_image_duplicates(directory, threshold, ui)
            groups = build_duplicate_groups(duplicates)
            total_deleted += report_and_delete("image", groups, directory, dry, yes, ui)

    if only in (None, "videos"):
        video_files = _collect_files(directory, VIDEO_EXTENSIONS)
        if not video_files:
            ui.output.print("No videos found.")
        else:
            ui.console.print(f"Scanning {len(video_files)} video(s)...")
            duplicates = find_video_duplicates(directory, threshold, ui)
            groups = build_duplicate_groups(duplicates)
            total_deleted += report_and_delete("video", groups, directory, dry, yes, ui)

    if dry:
        ui.output.print(f"\n\\[dry run] {total_deleted} file(s) would be deleted.")
    elif total_deleted:
        ui.console.print(f"\nRemoved {total_deleted} duplicate(s) total.")


@click.command()
@click.argument(
    "directory", type=click.Path(exists=True, file_okay=False, path_type=Path)
)
@click.option(
    "-t",
    "--threshold",
    type=int,
    default=1,
    show_default=True,
    help="Max hamming distance to consider as duplicate.",
)
@click.option("-n", "--dry-run", is_flag=True, help="List duplicates without deleting.")
@click.option(
    "--only",
    type=click.Choice(["images", "videos"]),
    default=None,
    help="Process only images or only videos.",
)
@click.option("--delete-empty", is_flag=True, help="Delete 0-byte media files.")
@click.option("-y", "--yes", is_flag=True, help="Skip confirmation prompt.")
@click.option("-q", "--quiet", is_flag=True, help="Suppress progress output.")
@click.option("-v", "--verbose", is_flag=True, help="Show verbose output.")
@click.option("--no-color", is_flag=True, help="Disable color output.")
def main(
    directory: Path,
    threshold: int,
    dry_run: bool,
    only: str | None,
    delete_empty: bool,
    yes: bool,
    quiet: bool,
    verbose: bool,
    no_color: bool,
) -> None:
    """Deduplicate images and videos using perceptual hashing."""
    ui = UI(
        console=Console(stderr=True, no_color=no_color),
        output=Console(no_color=no_color),
        quiet=quiet,
        verbose=verbose,
    )
    try:
        run(directory, threshold, dry_run, only, delete_empty, yes, ui)
    except KeyboardInterrupt:
        ui.console.print("\nInterrupted.")
        sys.exit(130)


if __name__ == "__main__":
    main()
