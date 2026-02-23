import shutil
from pathlib import Path
from unittest.mock import patch

import pytest

from imgdedup.main import build_duplicate_groups, main, parse_args, resolve_deletions


@pytest.fixture
def image_dir(tmp_path: Path) -> Path:
    fixtures = Path(__file__).parent / "fixtures"
    if fixtures.exists():
        shutil.copytree(fixtures, tmp_path / "images")
        return tmp_path / "images"
    d = tmp_path / "images"
    d.mkdir()
    return d


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


class TestParseArgs:
    def test_defaults(self):
        args = parse_args(["/some/dir"])
        assert args.directory == Path("/some/dir")
        assert args.method == "phash"
        assert args.threshold == 10
        assert args.dry_run is False

    def test_all_flags(self):
        args = parse_args(["/img", "-m", "ahash", "-t", "5", "-n"])
        assert args.method == "ahash"
        assert args.threshold == 5
        assert args.dry_run is True


class TestMain:
    def test_nonexistent_directory(self, tmp_path: Path):
        with pytest.raises(SystemExit):
            main([str(tmp_path / "nonexistent")])

    def test_empty_directory(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]):
        d = tmp_path / "empty"
        d.mkdir()
        main([str(d)])
        assert "No images found" in capsys.readouterr().out

    def _make_mock_hasher(self, fake_duplicates: dict[str, list[str]]):
        from unittest.mock import MagicMock

        mock_cls = MagicMock()
        mock_cls.return_value.find_duplicates.return_value = fake_duplicates
        return mock_cls

    def test_dry_run_no_deletion(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ):
        d = tmp_path / "imgs"
        d.mkdir()
        (d / "a.jpg").write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 100)
        (d / "b.jpg").write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 100)

        fake_duplicates = {"a.jpg": ["b.jpg"], "b.jpg": ["a.jpg"]}
        mock_hasher = self._make_mock_hasher(fake_duplicates)

        with patch.dict("imgdedup.main.HASHERS", {"phash": mock_hasher}):
            main([str(d), "-n"])

        assert (d / "a.jpg").exists()
        assert (d / "b.jpg").exists()
        output = capsys.readouterr().out
        assert "dry run" in output

    def test_actual_deletion(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]):
        d = tmp_path / "imgs"
        d.mkdir()
        (d / "a.jpg").write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 100)
        (d / "b.jpg").write_bytes(b"\xff\xd8\xff\xe0" + b"\x00" * 100)

        fake_duplicates = {"a.jpg": ["b.jpg"], "b.jpg": ["a.jpg"]}
        mock_hasher = self._make_mock_hasher(fake_duplicates)

        with patch.dict("imgdedup.main.HASHERS", {"phash": mock_hasher}):
            main([str(d)])

        assert (d / "a.jpg").exists()
        assert not (d / "b.jpg").exists()
        assert "Removed 1" in capsys.readouterr().out
