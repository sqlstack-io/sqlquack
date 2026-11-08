"""Test types for the DuckDB obligation build's --check."""

from __future__ import annotations

from dataclasses import dataclass


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
