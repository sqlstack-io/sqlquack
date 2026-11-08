"""Test types for the SQLite omission audit's review."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class RemovedRowTestCase:
    description: str
    table: str
    key: str
    removed: str
    expected_problems: list[str]


@dataclass(frozen=True)
class WalkTestCase:
    description: str
    unwalked_rules: frozenset[str]
    productions_read: int
    expected_problems: list[str]
