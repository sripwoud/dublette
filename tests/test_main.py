import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from my_pkg.main import main


def test_main_function_exists():
    """Test that main function is callable."""
    assert callable(main)


def test_main_runs_without_error(capsys):
    """Test that main function runs without error and produces output."""
    main()
    captured = capsys.readouterr()
    assert "Hello from py-template!" in captured.out
