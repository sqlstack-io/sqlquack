"""The DuckDB obligation build run in its own process, so no other dialect's hooks reach it."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPOSITORY: Path = Path(__file__).parents[4]
BUILD_MODULE: str = "scripts.build_syntax_obligations_duckdb"


def build_process(arguments: list[str]) -> subprocess.CompletedProcess[str]:
    """The build with `arguments`, run from the repository root in a fresh interpreter."""
    return subprocess.run(
        [sys.executable, "-m", BUILD_MODULE, *arguments],
        cwd=REPOSITORY,
        capture_output=True,
        text=True,
        check=False,
    )
