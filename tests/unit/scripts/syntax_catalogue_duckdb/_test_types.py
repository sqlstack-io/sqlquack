"""Test types for the DuckDB omission audit's review."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class UnseenTestCase:
    description: str
    removed_rows: frozenset[str]
    unwalked_rules: frozenset[str]
    expected_problems: list[str]
