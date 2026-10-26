"""Grouping and batching pending cases."""

from __future__ import annotations

from collections.abc import Iterator, Sequence

from sqlquack_record.recording.models import PendingCase
from sqlquack_record.recording.types import SessionParameters


def by_session(cases: Sequence[PendingCase]) -> dict[SessionParameters, list[PendingCase]]:
    """Cases grouped by the pinned and case parameters their session sets, in first-seen order."""
    groups: dict[SessionParameters, list[PendingCase]] = {}
    for case in cases:
        groups.setdefault(case.recording_parameters(), []).append(case)
    return groups


def batches(*, cases: Sequence[PendingCase], size: int) -> Iterator[Sequence[PendingCase]]:
    """Consecutive slices of at most `size` cases."""
    step: int = max(size, 1)
    for start in range(0, len(cases), step):
        yield cases[start : start + step]
