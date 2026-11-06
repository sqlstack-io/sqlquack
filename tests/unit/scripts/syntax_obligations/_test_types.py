"""Test types for the syntax obligation generator."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ShortenedTargetTestCase:
    description: str
    grammar: str
    expected_last_tokens: frozenset[str]
