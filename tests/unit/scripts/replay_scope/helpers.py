"""A small git repository for the replay scope's tests."""

from __future__ import annotations

import subprocess
from pathlib import Path


def committed_tree(*, root: Path, files: dict[str, str]) -> Path:
    """`root` as a git repository with `files` committed."""
    for name, text in files.items():
        (root / name).parent.mkdir(parents=True, exist_ok=True)
        (root / name).write_text(text)
    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    return commit_all(root=root)


def commit_all(*, root: Path) -> Path:
    """`root` with every change in it committed."""
    subprocess.run(["git", "add", "-A", "."], cwd=root, check=True)
    subprocess.run(
        [
            "git",
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.com",
            "commit",
            "-q",
            "-m",
            "files",
        ],
        cwd=root,
        check=True,
    )
    return root
