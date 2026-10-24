"""Load the pending case files `sqlquack pending` wrote."""

from __future__ import annotations

from pathlib import Path

from sqlquack_record.pending._helpers.parsing import is_case_file, parse_pending
from sqlquack_record.recording.models import PendingCase


def load_pending(directory: Path) -> list[PendingCase]:
    """Every pending case in `directory`, sorted by case reference."""
    if not directory.is_dir():
        return []
    cases: list[PendingCase] = [
        parse_pending(path) for path in sorted(directory.glob("*.json")) if is_case_file(path)
    ]
    return sorted(cases, key=lambda case: case.case_reference)
