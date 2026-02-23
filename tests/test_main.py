from pathlib import Path
from unittest.mock import MagicMock, patch

import imagehash
import numpy as np
import pytest

from imgdedup.main import (
    build_duplicate_groups,
    delete_empty_files,
    find_video_duplicates,
    main,
    parse_args,
    resolve_deletions,
)


def _make_hash(value: int) -> imagehash.ImageHash:
    bits = np.zeros((8, 8), dtype=bool)
    for i in range(64):
        if value & (1 << i):
            bits[i // 8][i % 8] = True
    return imagehash.ImageHash(bits)


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
    def test_finds_empty_media_files(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ):
        (tmp_path / "empty.jpg").touch()
        (tmp_path / "empty.mov").touch()
        (tmp_path / "nonempty.jpg").write_bytes(b"\xff\xd8" + b"\x00" * 10)
        (tmp_path / "empty.txt").touch()

        count = delete_empty_files(tmp_path, dry=True)
        assert count == 2
        assert (tmp_path / "empty.jpg").exists()
        assert "empty.jpg" in capsys.readouterr().out

    def test_deletes_empty_files(self, tmp_path: Path):
        (tmp_path / "empty.mp4").touch()
        (tmp_path / "good.mp4").write_bytes(b"\x00" * 10)

        count = delete_empty_files(tmp_path, dry=False)
        assert count == 1
        assert not (tmp_path / "empty.mp4").exists()
        assert (tmp_path / "good.mp4").exists()

    def test_no_empty_files(self, tmp_path: Path):
        (tmp_path / "a.jpg").write_bytes(b"\xff")
        assert delete_empty_files(tmp_path, dry=False) == 0


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

        with (
            patch("imgdedup.main._extract_frame_hash", side_effect=mock_extract),
            patch("imgdedup.main._get_ffmpeg", return_value="/usr/bin/ffmpeg"),
        ):
            result = find_video_duplicates(tmp_path, threshold=10)

        assert "b.mp4" in result["a.mp4"]
        assert "a.mp4" in result["b.mp4"]
        assert result["c.mp4"] == []

    def test_skips_corrupt_videos(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ):
        (tmp_path / "bad.mov").write_bytes(b"\x00" * 10)

        with (
            patch(
                "imgdedup.main._extract_frame_hash",
                side_effect=Exception("corrupt"),
            ),
            patch("imgdedup.main._get_ffmpeg", return_value="/usr/bin/ffmpeg"),
        ):
            result = find_video_duplicates(tmp_path, threshold=10)

        assert result == {}
        assert "skipping bad.mov" in capsys.readouterr().err


class TestParseArgs:
    def test_defaults(self):
        args = parse_args(["/some/dir"])
        assert args.directory == Path("/some/dir")
        assert args.method == "phash"
        assert args.threshold == 10
        assert args.dry_run is False
        assert args.only is None
        assert args.delete_empty is False

    def test_all_flags(self):
        args = parse_args(
            [
                "/img",
                "-m",
                "ahash",
                "-t",
                "5",
                "-n",
                "--only",
                "videos",
                "--delete-empty",
            ]
        )
        assert args.method == "ahash"
        assert args.threshold == 5
        assert args.dry_run is True
        assert args.only == "videos"
        assert args.delete_empty is True

    def test_only_images(self):
        args = parse_args(["/img", "--only", "images"])
        assert args.only == "images"


class TestMain:
    def test_nonexistent_directory(self, tmp_path: Path):
        with pytest.raises(SystemExit):
            main([str(tmp_path / "nonexistent")])

    def test_empty_directory(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]):
        d = tmp_path / "empty"
        d.mkdir()
        main([str(d)])
        out = capsys.readouterr().out
        assert "No images found" in out
        assert "No videos found" in out

    def _make_mock_hasher(self, fake_duplicates: dict[str, list[str]]):
        mock_cls = MagicMock()
        mock_cls.return_value.find_duplicates.return_value = fake_duplicates
        return mock_cls

    def test_dry_run_images(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]):
        d = tmp_path / "imgs"
        d.mkdir()
        (d / "a.jpg").write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 100)
        (d / "b.jpg").write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 100)

        fake_duplicates = {"a.jpg": ["b.jpg"], "b.jpg": ["a.jpg"]}
        mock_hasher = self._make_mock_hasher(fake_duplicates)

        with patch.dict("imgdedup.main.HASHERS", {"phash": mock_hasher}):
            main([str(d), "-n", "--only", "images"])

        assert (d / "a.jpg").exists()
        assert (d / "b.jpg").exists()
        output = capsys.readouterr().out
        assert "dry run" in output

    def test_actual_deletion_images(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ):
        d = tmp_path / "imgs"
        d.mkdir()
        (d / "a.jpg").write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 100)
        (d / "b.jpg").write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 100)

        fake_duplicates = {"a.jpg": ["b.jpg"], "b.jpg": ["a.jpg"]}
        mock_hasher = self._make_mock_hasher(fake_duplicates)

        with patch.dict("imgdedup.main.HASHERS", {"phash": mock_hasher}):
            main([str(d), "--only", "images"])

        assert (d / "a.jpg").exists()
        assert not (d / "b.jpg").exists()
        assert "Removed 1" in capsys.readouterr().out

    def test_only_videos_skips_images(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ):
        d = tmp_path / "mixed"
        d.mkdir()
        (d / "a.jpg").write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 100)
        main([str(d), "--only", "videos"])
        out = capsys.readouterr().out
        assert "No videos found" in out
        assert "image" not in out.lower()

    def test_only_images_skips_videos(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ):
        d = tmp_path / "mixed"
        d.mkdir()
        (d / "a.mp4").touch()
        main([str(d), "--only", "images"])
        out = capsys.readouterr().out
        assert "No images found" in out
        assert "video" not in out.lower()
