"""Test types for measuring guard raises over samples."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class MeasureTestCase:
    description: str
    values: tuple[str | None, ...]
    expression: str
    declared_type: str
    expected_status: str
    expected_raising_rows: int
    expected_reasons: tuple[tuple[str, str, int], ...]


@dataclass(frozen=True)
class NumberDetailTestCase:
    description: str
    text: str
    expected_reason: str
    expected_shape: str | None


@dataclass(frozen=True)
class RedactionTestCase:
    description: str
    values: tuple[str, ...]
    expected_absent: tuple[str, ...]
    expected_shape: str


@dataclass(frozen=True)
class CountTestCase:
    description: str
    expression: str
    options: tuple[str, ...]
    expected_count: tuple[int, bool, bool]
    expected_progress: str


@dataclass(frozen=True)
class ClassifyTestCase:
    description: str
    message: str
    expected_classification: tuple[str, str]
