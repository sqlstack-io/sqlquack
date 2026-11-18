"""Test types for the re-bound review check."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ReboundTestCase:
    description: str
    sql: str
    old_tree: str
    new_tree: str
    reviewed_tree: str
    expected_code: int
    expected_fragment: str
    note_field: str = "note"


@dataclass(frozen=True)
class StageReboundTestCase:
    description: str
    phase: str | None
    reviewed_stage: str | None
    old_case: dict[str, object]
    new_case: dict[str, object]
    expected_code: int
    expected_fragment: str
