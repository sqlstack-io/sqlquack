"""The Rust source files a transformation reads and rewrites."""

from __future__ import annotations

from pathlib import Path


def rust_files(*, roots: list[Path]) -> list[Path]:
    """Every `.rs` file under `roots` (files are taken as given), in a stable order."""
    files: list[Path] = []
    for root in roots:
        files.extend([root] if root.is_file() else sorted(root.rglob("*.rs")))
    return files
