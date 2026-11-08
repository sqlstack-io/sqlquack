"""Test types for the Databricks catalogue and its omission audit."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class UnseenTestCase:
    description: str
    removed_tokens: frozenset[str]
    unused_tokens: frozenset[str]
    extra_tokens: frozenset[str]
    expected_problems: list[str]


@dataclass(frozen=True)
class BlockTestCase:
    description: str
    found: tuple[str, ...]
    rows: tuple[str, ...]
    expected_problems: list[str]


@dataclass(frozen=True)
class ContextTestCase:
    description: str
    form: dict[str, object]
    expected_context: tuple[str, str]


@dataclass(frozen=True)
class PinTestCase:
    description: str
    fetched: str
    expected_differences: list[str]
