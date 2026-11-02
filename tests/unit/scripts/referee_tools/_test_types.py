"""Test types for the referee tooling."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class PendingTestCase:
    description: str
    selections: tuple[str, ...]
    expected_cases_from: str
    expected_environments: int


@dataclass(frozen=True)
class CategoryTestCase:
    description: str
    outcome: dict[str, object] | None
    parses: bool
    expected_category: str


@dataclass(frozen=True)
class ProjectTestCase:
    description: str
    model_sql: str
    expected_statements: int
    expected_skipped: dict[str, int]


@dataclass(frozen=True)
class CatalogueTestCase:
    description: str
    functions: list[dict[str, str]]
    expected_arities: dict[str, tuple[int, int | None]]
