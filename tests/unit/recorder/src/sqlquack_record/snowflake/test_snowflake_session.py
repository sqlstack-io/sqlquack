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
            expected_outcomes=("rows", "rows"),
            expected_single_runs=(),
            expected_files_written=4,
        ),
        BatchRecordingTestCase(
            description="a statement error wrapped by the multi-statement request reruns the batch "
            "one case at a time and records the real error",
            statements=_STATEMENTS,
            batch_error=(100132, "P0000"),
            statement_errors=(("SELECT 1 / 0", 100051, "22012"),),
            expected_outcomes=("rows", "error"),
            expected_single_runs=_STATEMENTS,
            expected_files_written=3,
        ),
        BatchRecordingTestCase(
            description="a wrapped error whose real error is not a case outcome stops the run "
            "without writing anything",
            statements=_STATEMENTS,
            batch_error=(100132, "P0000"),
            statement_errors=(("SELECT 1 / 0", 390114, "08001"),),
            expected_outcomes=None,
            expected_single_runs=_STATEMENTS,
            expected_files_written=0,
        ),
        BatchRecordingTestCase(
            description="a batch error that is not a case outcome stops the run at once",
            statements=_STATEMENTS,
            batch_error=(390114, "08001"),
            statement_errors=(),
            expected_outcomes=None,
            expected_single_runs=(),
            expected_files_written=0,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_batch_errors_when_recording_then_wrapped_errors_rerun_cases_individually(
    test_case: BatchRecordingTestCase, tmp_path: Path
) -> None:
    outcomes, single_runs, files_written = record_with_fake_connection(
        directory=tmp_path,
        statements=test_case.statements,
        batch_error=test_case.batch_error,
        statement_errors=test_case.statement_errors,
    )

    assert outcomes == test_case.expected_outcomes
    assert single_runs == test_case.expected_single_runs
    assert files_written == test_case.expected_files_written
