"""Grouping and batching pending cases."""

from __future__ import annotations

from collections.abc import Iterator, Sequence

from sqlquack_record.models import PendingCase, SessionValue


def by_session(
    cases: Sequence[PendingCase],
) -> dict[tuple[tuple[str, SessionValue], ...], list[PendingCase]]:
    """Cases grouped by their session parameters, in first-seen order."""
    groups: dict[tuple[tuple[str, SessionValue], ...], list[PendingCase]] = {}
    for case in cases:
        groups.setdefault(case.session_parameters, []).append(case)
    return groups


def batches(cases: Sequence[PendingCase], size: int) -> Iterator[Sequence[PendingCase]]:
    """Consecutive slices of at most `size` cases."""
    step: int = max(size, 1)
    for start in range(0, len(cases), step):
        yield cases[start : start + step]
