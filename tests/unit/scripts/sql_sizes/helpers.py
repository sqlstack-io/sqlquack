"""Sizes files for the size ratchet's tests."""

from __future__ import annotations

import json
from pathlib import Path


def sizes_file(path: Path, cases: dict[str, int]) -> Path:
    """`path` written as a sizes file holding `cases`."""
    path.write_text(json.dumps({"format": "sqlquack-sql-sizes/1", "cases": cases}))
    return path
