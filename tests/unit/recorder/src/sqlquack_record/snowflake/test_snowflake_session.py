"""Batch recording through the real Snowflake session over a fake connection."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.unit.recorder.src.sqlquack_record.snowflake._test_types import (
    BatchRecordingTestCase,
)
from tests.unit.recorder.src.sqlquack_record.snowflake.helpers import (
    record_with_fake_connection,
)

_STATEMENTS: tuple[str, ...] = ("SELECT 1", "SELECT 1 / 0")


@pytest.mark.parametrize(
    "test_case",
    (
        BatchRecordingTestCase(
            description="a batch without errors runs as one request",
            statements=_STATEMENTS,
            batch_error=None,
            statement_errors=(),
            json_results=frozenset(),
            expected_outcomes=("rows", "rows"),
            expected_single_runs=(),
            expected_files_written=4,
            expected_message="",
        ),
        BatchRecordingTestCase(
            description="a statement error wrapped by the multi-statement request reruns the batch "
            "one case at a time and records the real error",
            statements=_STATEMENTS,
            batch_error=(100132, "P0000"),
            json_results=frozenset(),
            statement_errors=(("SELECT 1 / 0", 100051, "22012"),),
            expected_outcomes=("rows", "error"),
            expected_single_runs=_STATEMENTS,
            expected_files_written=3,
            expected_message="",
        ),
        BatchRecordingTestCase(
            description="a wrapped error whose real error is not a case outcome stops the run "
            "without writing anything",
            statements=_STATEMENTS,
            batch_error=(100132, "P0000"),
            json_results=frozenset(),
            statement_errors=(("SELECT 1 / 0", 390114, "08001"),),
            expected_outcomes=None,
            expected_single_runs=_STATEMENTS,
            expected_files_written=0,
            expected_message="no result was written by this run",
        ),
        BatchRecordingTestCase(
            description="a privilege error depends on the role, so it stops the run",
            statements=_STATEMENTS,
            batch_error=(100132, "P0000"),
            json_results=frozenset(),
            statement_errors=(("SELECT 1 / 0", 3001, "42501"),),
            expected_outcomes=None,
            expected_single_runs=_STATEMENTS,
            expected_files_written=0,
            expected_message="no result was written by this run",
        ),
        BatchRecordingTestCase(
            description="a batch error that is not a case outcome stops the run at once",
            statements=_STATEMENTS,
            batch_error=(390114, "08001"),
            statement_errors=(),
            json_results=frozenset(),
            expected_outcomes=None,
            expected_single_runs=(),
            expected_files_written=0,
            expected_message="no result was written by this run",
        ),
        BatchRecordingTestCase(
            description="a JSON result in a batch reruns the batch case by case; a case still "
            "answered in JSON is refused with its reason and the others are recorded",
            statements=_STATEMENTS,
            batch_error=None,
            statement_errors=(),
            json_results=frozenset({"SELECT 1 / 0"}),
            expected_outcomes=("rows", "refused"),
            expected_single_runs=_STATEMENTS,
            expected_files_written=2,
            expected_message="fake.toml#1: Snowflake returned the result in JSON rather than Arrow",
        ),
        BatchRecordingTestCase(
            description="a later batch that stops the run reports the results already written",
            statements=("SELECT 1", "SELECT 2", "SELECT 1 / 0", "SELECT 3"),
            batch_error=None,
            statement_errors=(("SELECT 1 / 0;\nSELECT 3", 390114, "08001"),),
            json_results=frozenset(),
            expected_outcomes=None,
            expected_single_runs=(),
            expected_files_written=4,
            expected_message="2 results were written by this run before it stopped, the last "
            "for fake.toml#1",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_batch_errors_or_json_results_when_recording_then_cases_rerun_individually(
    test_case: BatchRecordingTestCase, tmp_path: Path
) -> None:
    outcomes, single_runs, files_written, message = record_with_fake_connection(
        directory=tmp_path,
        statements=test_case.statements,
        batch_error=test_case.batch_error,
        statement_errors=test_case.statement_errors,
        json_results=test_case.json_results,
    )

    assert outcomes == test_case.expected_outcomes
    assert single_runs == test_case.expected_single_runs
    assert files_written == test_case.expected_files_written
    assert test_case.expected_message in message
