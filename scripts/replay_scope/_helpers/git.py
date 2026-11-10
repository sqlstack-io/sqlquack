"""Tracked and changed files, from Git."""

from __future__ import annotations

import subprocess
from pathlib import Path


def tracked(*, root: Path, prefixes: tuple[str, ...]) -> list[str]:
    """The tracked files under `prefixes`, every tracked file when there are none, sorted."""
    return sorted(git_paths(root=root, arguments=("ls-files", "-z", "--", *prefixes)))


def changed(*, root: Path, base: str) -> list[str]:
    """The files changed since the merge base with `base`, committed or not, renames as two."""
    committed: set[str] = git_paths(
        root=root, arguments=("diff", "-z", "--name-only", "--no-renames", f"{base}...")
    )
    working: set[str] = git_paths(
        root=root, arguments=("diff", "-z", "--name-only", "--no-renames", "HEAD")
    )
    return sorted(committed | working)


def git_paths(*, root: Path, arguments: tuple[str, ...]) -> set[str]:
    """The NUL-separated paths a Git command prints."""
    output: bytes = subprocess.run(
        ("git", *arguments), cwd=root, check=True, capture_output=True
    ).stdout
    return {path for path in output.decode("utf-8").split("\0") if path}
