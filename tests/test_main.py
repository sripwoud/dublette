from io import StringIO
from pathlib import Path
from unittest.mock import patch

import imagehash
import numpy as np
from click.testing import CliRunner
from PIL import Image as PILImage
from rich.console import Console

from imgdedup.main import (
    UI,
    build_duplicate_groups,
    delete_empty_files,
    find_image_duplicates,
    find_video_duplicates,
    main,
    resolve_deletions,
)


def _make_hash(value: int) -> imagehash.ImageHash:
    bits = np.zeros((8, 8), dtype=bool)
    for i in range(64):
        if value & (1 << i):
            bits[i // 8][i % 8] = True
    return imagehash.ImageHash(bits)


def _make_image(path: Path) -> None:
    PILImage.new("RGB", (1, 1), color="red").save(path)


def _make_ui() -> tuple[UI, StringIO, StringIO]:
    stderr_buf = StringIO()
    stdout_buf = StringIO()
    ui = UI(
        console=Console(file=stderr_buf, no_color=True),
        output=Console(file=stdout_buf, no_color=True),
    )
    return ui, stderr_buf, stdout_buf


class TestBuildDuplicateGroups:
    def test_empty_input(self):
        assert build_duplicate_groups({}) == []

    def test_no_duplicates(self):
        result = build_duplicate_groups({"a.jpg": [], "b.jpg": [], "c.jpg": []})
        assert result == []

    def test_single_pair(self):
        groups = build_duplicate_groups({"a.jpg": ["b.jpg"], "b.jpg": ["a.jpg"]})
        assert len(groups) == 1
        assert groups[0] == {"a.jpg", "b.jpg"}

    def test_transitive_group(self):
        groups = build_duplicate_groups(
            {
                "a.jpg": ["b.jpg"],
                "b.jpg": ["a.jpg", "c.jpg"],
                "c.jpg": ["b.jpg"],
            }
        )
        assert len(groups) == 1
        assert groups[0] == {"a.jpg", "b.jpg", "c.jpg"}

    def test_two_separate_groups(self):
        groups = build_duplicate_groups(
            {
                "a.jpg": ["b.jpg"],
                "b.jpg": ["a.jpg"],
                "x.jpg": ["y.jpg"],
                "y.jpg": ["x.jpg"],
            }
        )
        assert len(groups) == 2
        names = [sorted(g) for g in groups]
        assert ["a.jpg", "b.jpg"] in names
        assert ["x.jpg", "y.jpg"] in names


class TestResolveDeletions:
    def test_keeps_first_alphabetically(self, tmp_path: Path):
        groups = [{"b.jpg", "a.jpg", "c.jpg"}]
        deletions = resolve_deletions(groups, tmp_path)
        assert deletions == [tmp_path / "b.jpg", tmp_path / "c.jpg"]

    def test_empty_groups(self, tmp_path: Path):
        assert resolve_deletions([], tmp_path) == []


class TestDeleteEmptyFiles:
    def test_finds_empty_media_files(self, tmp_path: Path):
        (tmp_path / "empty.jpg").touch()
        (tmp_path / "empty.mov").touch()
        (tmp_path / "nonempty.jpg").write_bytes(b"\xff\xd8" + b"\x00" * 10)
        (tmp_path / "empty.txt").touch()

        ui, _stderr_buf, stdout_buf = _make_ui()
        count = delete_empty_files(tmp_path, dry=True, ui=ui)
        assert count == 2
        assert (tmp_path / "empty.jpg").exists()
        output = stdout_buf.getvalue()
        assert "empty.jpg" in output
        assert "would delete" in output

    def test_deletes_empty_files(self, tmp_path: Path):
        (tmp_path / "empty.mp4").touch()
        (tmp_path / "good.mp4").write_bytes(b"\x00" * 10)

        ui, _stderr_buf, _stdout_buf = _make_ui()
        count = delete_empty_files(tmp_path, dry=False, ui=ui)
        assert count == 1
        assert not (tmp_path / "empty.mp4").exists()
        assert (tmp_path / "good.mp4").exists()

    def test_no_empty_files(self, tmp_path: Path):
        (tmp_path / "a.jpg").write_bytes(b"\xff")
        ui, _stderr_buf, _stdout_buf = _make_ui()
        assert delete_empty_files(tmp_path, dry=False, ui=ui) == 0

    def test_recurses_into_subdirs(self, tmp_path: Path):
        sub = tmp_path / "sub"
        sub.mkdir()
        (sub / "empty.png").touch()
        (tmp_path / "empty.jpg").touch()

        ui, _stderr_buf, _stdout_buf = _make_ui()
        count = delete_empty_files(tmp_path, dry=True, ui=ui)
        assert count == 2


class TestFindImageDuplicates:
    def test_pairwise_comparison(self, tmp_path: Path):
        hash_a = _make_hash(0)
        hash_b = _make_hash(1)
        hash_c = _make_hash(0xFFFF)

        _make_image(tmp_path / "a.jpg")
        _make_image(tmp_path / "b.jpg")
        _make_image(tmp_path / "c.jpg")

        hash_map = {"a.jpg": hash_a, "b.jpg": hash_b, "c.jpg": hash_c}

        ui, _stderr_buf, _stdout_buf = _make_ui()
        with patch(
            "imgdedup.main.imagehash.phash",
            side_effect=lambda img: hash_map[Path(img.filename).name],
        ):
            result = find_image_duplicates(tmp_path, threshold=9, ui=ui)

        assert "b.jpg" in result["a.jpg"]
        assert "a.jpg" in result["b.jpg"]
        assert result["c.jpg"] == []

    def test_cross_folder_duplicates(self, tmp_path: Path):
        sub = tmp_path / "sub"
        sub.mkdir()
        _make_image(tmp_path / "a.jpg")
        _make_image(sub / "b.jpg")

        same_hash = _make_hash(42)

        ui, _stderr_buf, _stdout_buf = _make_ui()
        with patch("imgdedup.main.imagehash.phash", return_value=same_hash):
            result = find_image_duplicates(tmp_path, threshold=9, ui=ui)

        keys = list(result.keys())
        assert len(keys) == 2
        assert any("sub" in k for k in keys)


class TestFindVideoDuplicates:
    def test_pairwise_comparison(self, tmp_path: Path):
        (tmp_path / "a.mp4").write_bytes(b"\x00" * 10)
        (tmp_path / "b.mp4").write_bytes(b"\x00" * 10)
        (tmp_path / "c.mp4").write_bytes(b"\x00" * 10)

        hash_a = _make_hash(0)
        hash_b = _make_hash(1)
        hash_c = _make_hash(0xFFFF)

        def mock_extract(path, ffmpeg):
            name = Path(path).name
            return {"a.mp4": hash_a, "b.mp4": hash_b, "c.mp4": hash_c}[name]

        ui, _stderr_buf, _stdout_buf = _make_ui()
        with (
            patch("imgdedup.main._extract_frame_hash", side_effect=mock_extract),
            patch("imgdedup.main._get_ffmpeg", return_value="/usr/bin/ffmpeg"),
        ):
            result = find_video_duplicates(tmp_path, threshold=9, ui=ui)

        assert "b.mp4" in result["a.mp4"]
        assert "a.mp4" in result["b.mp4"]
        assert result["c.mp4"] == []

    def test_skips_corrupt_videos(self, tmp_path: Path):
        (tmp_path / "bad.mov").write_bytes(b"\x00" * 10)

        ui, stderr_buf, _stdout_buf = _make_ui()
        with (
            patch(
                "imgdedup.main._extract_frame_hash",
                side_effect=Exception("corrupt"),
            ),
            patch("imgdedup.main._get_ffmpeg", return_value="/usr/bin/ffmpeg"),
        ):
            result = find_video_duplicates(tmp_path, threshold=9, ui=ui)

        assert result == {}
        assert "skipping bad.mov" in stderr_buf.getvalue()


class TestMain:
    def test_nonexistent_directory(self, tmp_path: Path):
        runner = CliRunner()
        result = runner.invoke(main, [str(tmp_path / "nonexistent")])
        assert result.exit_code == 2

    def test_empty_directory(self, tmp_path: Path):
        d = tmp_path / "empty"
        d.mkdir()
        runner = CliRunner()
        result = runner.invoke(main, [str(d)])
        assert result.exit_code == 0
        assert "No images found" in result.output
        assert "No videos found" in result.output

    def test_only_videos_skips_images(self, tmp_path: Path):
        d = tmp_path / "mixed"
        d.mkdir()
        (d / "a.jpg").write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 100)
        runner = CliRunner()
        result = runner.invoke(main, [str(d), "--only", "videos"])
        assert result.exit_code == 0
        assert "No videos found" in result.output
        assert "image" not in result.output.lower()

    def test_only_images_skips_videos(self, tmp_path: Path):
        d = tmp_path / "mixed"
        d.mkdir()
        (d / "a.mp4").write_bytes(b"\x00" * 10)
        runner = CliRunner()
        result = runner.invoke(main, [str(d), "--only", "images"])
        assert result.exit_code == 0
        assert "No images found" in result.output
        assert "video" not in result.output.lower()

    def test_no_color_flag(self, tmp_path: Path):
        d = tmp_path / "nc"
        d.mkdir()
        runner = CliRunner()
        result = runner.invoke(main, [str(d), "--no-color"])
        assert result.exit_code == 0
        assert "\x1b[" not in result.output

    def test_quiet_suppresses_progress(self, tmp_path: Path):
        _make_image(tmp_path / "a.jpg")
        _make_image(tmp_path / "b.jpg")
        runner = CliRunner()
        result = runner.invoke(main, [str(tmp_path), "--quiet", "--only", "images"])
        assert result.exit_code == 0
        assert "Hashing" not in result.output
        assert "Comparing" not in result.output
