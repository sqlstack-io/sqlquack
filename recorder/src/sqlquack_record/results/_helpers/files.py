"""Exclusive, append-only file creation."""

from __future__ import annotations

import os
from pathlib import Path

from sqlquack_record.errors import RecorderError


def create_exclusively(path: Path, content: bytes) -> None:
    """Write `content` to `path`, failing if anything already exists there."""
    try:
        descriptor: int = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    except FileExistsError as error:
        raise RecorderError(f"{path.name} already exists; results are append-only") from error
    with os.fdopen(descriptor, "wb") as handle:
        handle.write(content)
