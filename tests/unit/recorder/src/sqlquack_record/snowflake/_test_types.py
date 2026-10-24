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
    expected_outcomes: tuple[str, ...] | None
    expected_single_runs: tuple[str, ...]
    expected_files_written: int
