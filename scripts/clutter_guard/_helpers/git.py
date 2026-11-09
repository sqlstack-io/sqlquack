"""Listing tracked files from Git."""

from __future__ import annotations

import subprocess
from pathlib import Path


def tracked_paths(*, root: Path) -> list[str]:
    """Every path Git tracks in `root`, relative to it."""
    output: bytes = subprocess.run(
        ("git", "ls-files", "-z"), cwd=root, check=True, capture_output=True
    ).stdout
    return [path for path in output.decode("utf-8").split("\0") if path]
