"""Test types for the BigQuery referee."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from scripts.bigquery_referee.types import Row


@dataclass(frozen=True)
class SubstituteTestCase:
    description: str
    sql: str
    expected_sql: str


@dataclass(frozen=True)
class MappingTestCase:
    description: str
    status: int
    body: Row
    expected_phase: str
    expected_code: str | None
    expected_message: str | None
    expected_unmapped: tuple[str, ...] = ()


@dataclass(frozen=True)
class CapTestCase:
    description: str
    case_id: str
    sql: str
    expected_dry_runs: list[bool]


@dataclass(frozen=True)
class StopTestCase:
    description: str
    status: int
    body: Row
    expected_match: str


@dataclass(frozen=True)
class RetryTestCase:
    description: str
    replies: list[tuple[int, Row]]
    expected_status: int
    expected_sleeps: int


@dataclass(frozen=True)
class NormaliseTestCase:
    description: str
    sql: str
    message: str
    expected_message: str


@dataclass(frozen=True)
class LeakTestCase:
    description: str
    text: str
    expected_leak: bool


@dataclass(frozen=True)
class PolicyTestCase:
    description: str
    case_id: str
    sql: str
    expected_mode: str


@dataclass(frozen=True)
class TargetTestCase:
    description: str
    dataset: str
    expected_held: str


@dataclass(frozen=True)
class DatasetTestCase:
    description: str
    dataset: str
    expected_calls: int = 0


@dataclass(frozen=True)
class RefusalTableTestCase:
    description: str
    grammar: str
    expected_patterns: list[str]


@dataclass(frozen=True)
class RefusalMessageTestCase:
    description: str
    message: str
    expected_code: str


@dataclass(frozen=True)
class FailedRunTestCase:
    description: str
    dry: Callable[[str], tuple[int, Row]]
    cases: list[Row]
    expected_error: type[Exception]
    expected_match: str


@dataclass(frozen=True)
class RecordTestCase:
    description: str
    cases: list[Row]
    expected_phases: dict[str, int]
    expected_held: int
    expected_compared: str


@dataclass(frozen=True)
class HeldTestCase:
    description: str
    cases: list[Row]
    expected_phases: dict[str, int]
    expected_held: dict[str, int]


@dataclass(frozen=True)
class ProjectTestCase:
    description: str
    value: str
    expected_error: str


@dataclass(frozen=True)
class CaseDatasetsTestCase:
    description: str
    cases: list[Row]
    expected_own: set[str]


@dataclass(frozen=True)
class TimedTestCase:
    description: str
    polls_before_done: int | None
    stop_message: str
    expected_code: str
    expected_cancels: int
