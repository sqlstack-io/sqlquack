"""Test types for the syntax obligation generator."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ShortenedTargetTestCase:
    description: str
    grammar: str
    expected_last_tokens: frozenset[str]


@dataclass(frozen=True)
class SetupBindingTestCase:
    description: str
    receipt_sql_sha256: str
    report_input_sql_sha256: str
    expected_setup: tuple[str, ...]


@dataclass(frozen=True)
class DisputedFormMarkTestCase:
    description: str
    sql: str
    expected_mark: str
    expected_expect: str
