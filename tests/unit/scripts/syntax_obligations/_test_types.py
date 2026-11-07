"""Test types for the syntax obligation generator."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ShortenedTargetTestCase:
    description: str
    grammar: str
    expected_last_tokens: frozenset[str]


@dataclass(frozen=True)
class RecordedSetupTestCase:
    description: str
    input_sql_sha256: str
    expected_setups: dict[tuple[str, str], list[str]]
