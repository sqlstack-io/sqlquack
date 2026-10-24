"""Recording pending cases against a fake warehouse session."""

from __future__ import annotations

from pathlib import Path

import pytest

from recorder.tests.recording._test_types import RecordTestCase
from recorder.tests.recording.helpers import record

_UTC: tuple[tuple[str, bool | int | str], ...] = (("TIMEZONE", "UTC"),)
_WEEKS: tuple[tuple[str, bool | int | str], ...] = (("TIMEZONE", "UTC"), ("WEEK_START", 1))


@pytest.mark.parametrize(
    "test_case",
    (
        RecordTestCase(
            description="rows are stored as the connector's Arrow table, unconverted",
            statements=("SELECT 1", "SELECT 22"),
            failing=frozenset(),
            parameters=(_UTC,),
            preexisting=(),
            expected_recorded=2,
            expected_errors=0,
            expected_sessions=1,
            expected_batches=1,
            expected_outcomes=("rows", "rows"),
        ),
        RecordTestCase(
            description="a batch with a SQL error is rerun case by case and the error recorded",
            statements=("SELECT 1", "SELECT 1 / 0"),
            failing=frozenset({"SELECT 1 / 0"}),
            parameters=(_UTC,),
            preexisting=(),
            expected_recorded=2,
            expected_errors=1,
            expected_sessions=1,
            expected_batches=1,
            expected_outcomes=("rows", "error"),
        ),
        RecordTestCase(
            description="an existing result is never re-recorded",
            statements=("SELECT 1", "SELECT 22"),
            failing=frozenset(),
            parameters=(_UTC,),
            preexisting=("SELECT 1",),
            expected_recorded=1,
            expected_errors=0,
            expected_sessions=1,
            expected_batches=0,
            expected_outcomes=("preexisting", "rows"),
        ),
        RecordTestCase(
            description="each set of session parameters gets its own session",
            statements=("SELECT 1", "SELECT 22"),
            failing=frozenset(),
            parameters=(_UTC, _WEEKS),
            preexisting=(),
            expected_recorded=2,
            expected_errors=0,
            expected_sessions=2,
            expected_batches=0,
            expected_outcomes=("rows", "rows"),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_pending_cases_when_recording_then_results_are_stored_once_and_unconverted(
    test_case: RecordTestCase, tmp_path: Path
) -> None:
    recorded, errors, log, outcomes = record(
        tmp_path,
        test_case.statements,
        test_case.failing,
        test_case.parameters,
        test_case.preexisting,
    )

    assert recorded == test_case.expected_recorded
    assert errors == test_case.expected_errors
    assert log.count("open") == test_case.expected_sessions
    assert log.count("batch") == test_case.expected_batches
    assert tuple(outcomes) == test_case.expected_outcomes
