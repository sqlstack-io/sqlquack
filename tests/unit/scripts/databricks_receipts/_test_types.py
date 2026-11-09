"""Test types for the Databricks referee."""

from __future__ import annotations

from dataclasses import dataclass

from scripts.databricks_receipts.types import Row


@dataclass(frozen=True)
class PhaseTestCase:
    description: str
    message: str
    sqlstate: str
    expected_phase: str | None
    expected_parse_only: str | None


@dataclass(frozen=True)
class JudgedTypeTestCase:
    description: str
    message: str
    sqlstate: str
    expected_type: type


@dataclass(frozen=True)
class StatelessTestCase:
    description: str
    reviewed_message: str
    other_message: str
    expected_phase: str
    expected_engine_code: str | None


@dataclass(frozen=True)
class RefusalTestCase:
    description: str
    message: str
    sqlstate: str
    expected_error: str


@dataclass(frozen=True)
class PublishTestCase:
    description: str
    messages: tuple[str, ...]
    expected_published: tuple[str, ...]


@dataclass(frozen=True)
class PhaseCodeTestCase:
    description: str
    message: str
    sqlstate: str
    expected_phase: str
    expected_engine_code: str
    expected_parse_only: str | None


@dataclass(frozen=True)
class PhaseTableTestCase:
    description: str
    expected_absent_phase: str


@dataclass(frozen=True)
class ScopeTestCase:
    description: str
    sql: str
    expected_held: bool


@dataclass(frozen=True)
class HeldRunTestCase:
    description: str
    cases: tuple[Row, ...]
    expected_error: str
    expected_reports: int


@dataclass(frozen=True)
class CleanupTestCase:
    description: str
    sql: str
    expected_receipts: int
    expected_phases: list[tuple[str, str]]
    expected_cleanup: list[Row]


@dataclass(frozen=True)
class StopTestCase:
    description: str
    sql: str
    expected_error: str


@dataclass(frozen=True)
class ErrorTestCase:
    description: str
    expected_error: str


@dataclass(frozen=True)
class SequentialHeldTestCase:
    description: str
    sql: str
    expected_error: str
    expected_deleted: list[str]


@dataclass(frozen=True)
class PolicySweepTestCase:
    description: str
    sql: str
    expected_drops: int


@dataclass(frozen=True)
class RecoverTestCase:
    description: str
    expected_status: int
    expected_unsettled: list[str]
    expected_left: list[str]


@dataclass(frozen=True)
class PolicyAnswerTestCase:
    description: str
    message: str
    expected_receipts: int


@dataclass(frozen=True)
class PolicyRefusalTestCase:
    description: str
    message: str
    expected_error: str


@dataclass(frozen=True)
class ResumeTestCase:
    description: str
    expected_receipts: int
    expected_events: list[str]


@dataclass(frozen=True)
class ResumeRefusalTestCase:
    description: str
    change: str
    seed: int | None
    expected_error: str


@dataclass(frozen=True)
class UndecidedTestCase:
    description: str
    expected_receipts: int
    expected_undecided: list[str]
    expected_reasons: dict[str, str]


@dataclass(frozen=True)
class CheckTestCase:
    description: str
    failing_checks: frozenset[int]
    expected_error: str
    expected_events: list[str]
    expected_receipts: int


@dataclass(frozen=True)
class RefinishTestCase:
    description: str
    expected_receipts: int
    expected_phase: str
    expected_query_id: str


@dataclass(frozen=True)
class HistoryTestCase:
    description: str
    messages: tuple[str, str]
    expected_error: str


@dataclass(frozen=True)
class TokenTestCase:
    description: str
    expected_reply: Row
    expected_sent: list[str]


@dataclass(frozen=True)
class ResendTestCase:
    description: str
    expected_statement: list[str]
    expected_check: list[str]


@dataclass(frozen=True)
class LegacySetupTestCase:
    description: str
    statement: str
    expected_undecided: bool


@dataclass(frozen=True)
class DeadlineTestCase:
    description: str
    seconds: str
    expected_error: str
    expected_calls: list[str]
