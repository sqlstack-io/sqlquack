"""Pending case files in the format `sqlquack pending` writes."""

from __future__ import annotations

from pathlib import Path

from sqlquack_record.pending.main.load_pending import load_pending
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
