"""Test types for the Databricks omission audit's review."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class UnseenTestCase:
    description: str
    removed_tokens: frozenset[str]
    unused_tokens: frozenset[str]
    extra_tokens: frozenset[str]
    expected_problems: list[str]
