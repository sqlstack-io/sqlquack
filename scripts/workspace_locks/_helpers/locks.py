"""Resolve a workspace from its lockfile alone."""

from __future__ import annotations

import subprocess
from pathlib import Path


def locked_resolution_error(*, workspace: Path) -> str:
    """Cargo's error when the workspace cannot resolve without changing its lockfile, else ""."""
    completed: subprocess.CompletedProcess[str] = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=workspace,
        check=False,
        capture_output=True,
        text=True,
    )
    lines: list[str] = completed.stderr.strip().splitlines()
    errors: list[str] = [line for line in lines if line.startswith("error:")] or lines[-1:]
    return "" if completed.returncode == 0 else (errors or ["cargo metadata failed"])[0]
