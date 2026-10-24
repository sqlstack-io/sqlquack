"""Test types for recording pending cases."""

from __future__ import annotations

from dataclasses import dataclass

# What was stored for one case: the outcome, whether the Arrow file matches the metadata hash,
# and whether the stored rows equal the warehouse's Arrow table.
type RecordedFile = tuple[str, bool, bool]


@dataclass(frozen=True)
class RecordTestCase:
    description: str
    statements: tuple[str, ...]
    failing: frozenset[str]
    batch_fails: bool
    parameters: tuple[tuple[tuple[str, bool | int | str], ...], ...]
    preexisting: tuple[str, ...]
    expected_recorded: int
    expected_failures: int
    expected_log: tuple[str, ...]
    expected_files: tuple[RecordedFile, ...]
