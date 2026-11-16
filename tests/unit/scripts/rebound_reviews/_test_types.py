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
