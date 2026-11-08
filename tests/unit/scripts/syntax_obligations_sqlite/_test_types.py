"""Test types for the SQLite obligation build's tombstone guard and --check."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class TombstoneTestCase:
    description: str
    limits: str
    retired: str
    expected_error: str
    expected_present: bool
    expected_unchanged: bool


@dataclass(frozen=True)
class CheckTestCase:
    description: str
    relative: str
    old: str
    new: str
    expected_code: int
    expected_stale: int


@dataclass(frozen=True)
class StrayTestCase:
    description: str
    relative: str
    expected_code: int
