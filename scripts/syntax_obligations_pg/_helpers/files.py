"""The generated files of a root, kept so a failed build leaves the root as it was."""

from __future__ import annotations

import os
from pathlib import Path

from scripts.syntax_obligations_pg.constants import CASES_DIR, OBLIGATIONS_DIR


def snapshot(root: Path) -> dict[Path, bytes]:
    """The bytes of every generated file under obligations/ and cases/, at any depth."""
    kept: dict[Path, bytes] = {}
    for directory in (OBLIGATIONS_DIR, CASES_DIR):
        for path in sorted((root / directory).rglob("*")):
            if path.is_file():
                kept[path] = path.read_bytes()
    return kept


def restore(*, root: Path, kept: dict[Path, bytes]) -> None:
    """Put the generated files back as `snapshot` read them, removing new files and directories."""
    for path, data in kept.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        staged: Path = path.with_name(f".{path.name}.restore")
        staged.write_bytes(data)
        os.replace(staged, path)
    for directory in (OBLIGATIONS_DIR, CASES_DIR):
        top: Path = root / directory
        for path in sorted(top.rglob("*"), reverse=True):
            if path.is_file() and path not in kept:
                path.unlink()
            elif path.is_dir() and not any(path.iterdir()):
                path.rmdir()
