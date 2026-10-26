"""Pending case files in the format `sqlquack pending` writes."""

from __future__ import annotations

from pathlib import Path

from sqlquack_record.pending.main.evidence_directories import evidence_directories
from sqlquack_record.pending.main.load_pending import load_pending
from sqlquack_record.recording.constants import REPOSITORY_MARKER
from sqlquack_record.recording.exceptions import RecorderError

CASE_HASH: str = "29c5ebb55926dab5055ab46ba08e010c775eab2da0124ac636ed33d87f56376c"
PENDING_FILE: str = """{
  "format": 1,
  "case_hash": "29c5ebb55926dab5055ab46ba08e010c775eab2da0124ac636ed33d87f56376c",
  "case_reference": "numeric/addition.toml#mixed_scales",
  "source_dialect": "snowflake",
  "session_parameters": {
    "TIMEZONE": "UTC",
    "USE_CACHED_RESULT": false,
    "WEEK_START": 0
  },
  "source_sql": "SELECT 1 AS ordinal",
  "duckdb_version": "v2.0.0"
}
"""


def references(*, directory: Path, file_name: str, text: str) -> tuple[str, ...] | None:
    """The case references loaded from one pending file, or `None` when it is rejected."""
    (directory / file_name).write_text(text, encoding="utf-8")
    try:
        return tuple(case.case_reference for case in load_pending(directory))
    except RecorderError:
        return None


def missing_directory_references(directory: Path) -> tuple[str, ...] | None:
    """The references loaded from a directory that does not exist, or `None` when refused."""
    try:
        return tuple(case.case_reference for case in load_pending(directory / "missing"))
    except RecorderError:
        return None


def resolved_directories(
    *,
    root: Path,
    marker_at: str | None,
    working_directory: str,
    pending: str | None,
    results: str | None,
) -> tuple[str, str] | None:
    """The resolved directories relative to `root`, or `None` when resolution fails."""
    for directory in filter(None, (marker_at,)):
        (root / directory).mkdir(parents=True, exist_ok=True)
        (root / directory / REPOSITORY_MARKER).write_text("schema_version = 1\n", encoding="utf-8")
    start: Path = root / working_directory
    start.mkdir(parents=True, exist_ok=True)
    explicit: dict[str, Path | None] = {
        name: next(map(root.joinpath, filter(None, (value,))), None)
        for name, value in (("pending", pending), ("results", results))
    }
    try:
        found: tuple[Path, Path] = evidence_directories(
            pending=explicit["pending"], results=explicit["results"], working_directory=start
        )
    except RecorderError:
        return None
    return (found[0].relative_to(root).as_posix(), found[1].relative_to(root).as_posix())
