"""Resolve the pending and results directories against the repository root."""

from __future__ import annotations

from pathlib import Path

from sqlquack_record.recording.constants import (
    DEFAULT_PENDING_DIR,
    DEFAULT_RESULTS_DIR,
    REPOSITORY_MARKER,
)
from sqlquack_record.recording.exceptions import RecorderError


def evidence_directories(
    *, pending: Path | None, results: Path | None, working_directory: Path
) -> tuple[Path, Path]:
    """The given directories, with defaults under the repository holding the working directory."""
    if pending is not None and results is not None:
        return pending, results
    root: Path | None = next(
        (
            directory
            for directory in (working_directory, *working_directory.parents)
            if (directory / REPOSITORY_MARKER).is_file()
        ),
        None,
    )
    if root is None:
        raise RecorderError(
            f"no {REPOSITORY_MARKER} in {working_directory} or any parent; run inside the "
            "SQLQuack repository or pass --pending and --results"
        )
    return (pending or root / DEFAULT_PENDING_DIR, results or root / DEFAULT_RESULTS_DIR)
