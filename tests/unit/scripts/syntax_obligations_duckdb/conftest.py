"""The committed DuckDB root, built once with no receipts, for the --check tests."""

from __future__ import annotations

import shutil
from pathlib import Path

import pytest

from tests.unit.scripts.syntax_obligations_duckdb.helpers import build_process

ROOT: Path = Path(__file__).parents[4] / "coverage" / "syntax-duckdb"
COPIED: tuple[str, ...] = ("catalogue", "obligations", "limits.toml", "retired.toml")


@pytest.fixture(scope="module")
def built_root(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """The committed catalogue, obligations, limits and retired ids, built once in a subprocess."""
    root: Path = tmp_path_factory.mktemp("built") / "root"
    root.mkdir()
    for name in COPIED:
        source: Path = ROOT / name
        if source.is_dir():
            shutil.copytree(source, root / name)
        else:
            shutil.copy(source, root / name)
    build_process(["--root", str(root)]).check_returncode()
    return root
