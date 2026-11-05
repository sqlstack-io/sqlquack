"""Test types for connection arguments."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ConnectArgumentsTestCase:
    description: str
    connection_name: str | None
    warehouse: str | None
    overrides: tuple[tuple[str, str], ...]
    expected_arguments: dict[str, object] | None


@dataclass(frozen=True)
class BatchRecordingTestCase:
    description: str
    statements: tuple[str, ...]
    batch_error: tuple[int, str] | None
    statement_errors: tuple[tuple[str, int, str], ...]
    json_results: frozenset[str]
    overflow_results: frozenset[str]
    truncated_results: frozenset[str]
    null_results: frozenset[str]
    expected_outcomes: tuple[str, ...] | None
    expected_single_runs: tuple[str, ...]
    expected_files_written: int
    expected_message: str


@dataclass(frozen=True)
class ErrorMessageTestCase:
    description: str
    message: str
    sqlstate: str
    query_id: str | None
    expected_message: str | None


@dataclass(frozen=True)
class RecordableErrorTestCase:
    description: str
    errno: int
    sqlstate: str
    expected_recordable: bool


@dataclass(frozen=True)
class SubMicrosecondTestCase:
    description: str
    type_name: str
    names: tuple[str, ...]
    nanoseconds: tuple[tuple[int | None, ...], ...]
    check_answer: str
    expected_checked: tuple[bool, ...] | None
    expected_message: str | None
    expected_checks: int
