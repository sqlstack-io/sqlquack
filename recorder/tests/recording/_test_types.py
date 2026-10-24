"""Test types for recording pending cases."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class RecordTestCase:
    description: str
    statements: tuple[str, ...]
    failing: frozenset[str]
    parameters: tuple[tuple[tuple[str, bool | int | str], ...], ...]
    preexisting: tuple[str, ...]
    expected_recorded: int
    expected_errors: int
    expected_sessions: int
    expected_batches: int
    expected_outcomes: tuple[str, ...]
