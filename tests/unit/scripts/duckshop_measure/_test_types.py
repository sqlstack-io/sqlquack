"""Test types for the DuckShop measures."""

from __future__ import annotations

import datetime
from dataclasses import dataclass, field


@dataclass(frozen=True)
class SchemaTestCase:
    description: str
    setup_sql: tuple[str, ...]
    expected_schema: dict[str, list[list[str]]]


@dataclass(frozen=True)
class OutcomeTestCase:
    description: str
    converted: bool
    python_functions: bool
    sql: str
    expected_outcome: str
    expected_details: tuple[str, ...] | None


@dataclass(frozen=True)
class RunTestCase:
    description: str
    files: tuple[tuple[str, bool], ...]
    expected_outcomes: dict[str, int]
    expected_order: tuple[str, ...]


@dataclass(frozen=True)
class OrderTestCase:
    description: str
    sql: str
    expected_ordered: bool


@dataclass(frozen=True)
class ModelTestCase:
    description: str
    sidecar: dict[str, object]
    refusals: tuple[str, ...] | None
    compared: dict[str, str] | None
    error: str | None
    expected_outcome: str
    declared: tuple[str, ...] = ()
    unloadable: dict[str, str] = field(default_factory=dict)


@dataclass(frozen=True)
class LoadTestCase:
    description: str
    rows: tuple[datetime.datetime, ...]
    expected_unloadable: tuple[str, ...]
    expected_loaded: tuple[str, ...]


@dataclass(frozen=True)
class SnowflakeTestCase:
    description: str
    policy: str
    expected_compared: tuple[str, ...]
    expected_scored: int
    expected_outcomes: dict[str, list[str]]
