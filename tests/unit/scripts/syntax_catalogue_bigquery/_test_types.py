"""Test types for the BigQuery omission audit's review."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ReviewTestCase:
    description: str
    removed: tuple[tuple[str, str, str], ...]
    unwalked_rules: frozenset[str]
    expected_problems: list[str]


@dataclass(frozen=True)
class DocsTestCase:
    description: str
    extra_streams: list[tuple[str, list[str]]]
    expected_problems: list[str]


@dataclass(frozen=True)
class RetrievalTestCase:
    description: str
    page_id: str
    page_sha256: str
    expected_retrieved_utc: str
