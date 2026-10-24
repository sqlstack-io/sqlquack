"""Recording pending cases against a fake warehouse session."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.unit.recorder.src.sqlquack_record.recording._test_types import (
    RecordedFile,
    RecordTestCase,
)
from tests.unit.recorder.src.sqlquack_record.recording.helpers import record

_UTC: tuple[tuple[str, bool | int | str], ...] = (("TIMEZONE", "UTC"),)
_WEEKS: tuple[tuple[str, bool | int | str], ...] = (("TIMEZONE", "UTC"), ("WEEK_START", 1))
_ROWS: RecordedFile = ("rows", True, True)
_ERROR: RecordedFile = ("error", True, True)
_EXISTING: RecordedFile = ("preexisting", True, True)


@pytest.mark.parametrize(
    "test_case",
    (
        RecordTestCase(
            description="rows are stored as the connector's Arrow table in one batch, unconverted",
            statements=("SELECT 1", "SELECT 22"),
            failing=frozenset(),
            batch_fails=False,
            parameters=(_UTC,),
            preexisting=(),
            expected_recorded=2,
            expected_failures=0,
            expected_log=("open", "batch", "close"),
            expected_files=(_ROWS, _ROWS),
        ),
        RecordTestCase(
            description="a batch with a SQL error is rerun case by case and the error recorded",
            statements=("SELECT 1", "SELECT 1 / 0"),
            failing=frozenset({"SELECT 1 / 0"}),
            batch_fails=True,
            parameters=(_UTC,),
            preexisting=(),
            expected_recorded=2,
            expected_failures=1,
            expected_log=("open", "batch", "one", "one", "close"),
            expected_files=(_ROWS, _ERROR),
        ),
        RecordTestCase(
            description="an existing result is never re-recorded",
            statements=("SELECT 1", "SELECT 22"),
            failing=frozenset(),
            batch_fails=False,
            parameters=(_UTC,),
            preexisting=("SELECT 1",),
            expected_recorded=1,
            expected_failures=0,
            expected_log=("open", "one", "close"),
            expected_files=(_EXISTING, _ROWS),
        ),
        RecordTestCase(
            description="each set of session parameters gets its own session",
            statements=("SELECT 1", "SELECT 22"),
            failing=frozenset(),
            batch_fails=False,
            parameters=(_UTC, _WEEKS),
            preexisting=(),
            expected_recorded=2,
            expected_failures=0,
            expected_log=("open", "one", "close", "open", "one", "close"),
            expected_files=(_ROWS, _ROWS),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_pending_cases_when_recording_then_results_are_stored_once_and_unconverted(
    test_case: RecordTestCase, tmp_path: Path
) -> None:
    recorded, failures, log, files = record(
        directory=tmp_path,
        statements=test_case.statements,
        failing=test_case.failing,
        batch_fails=test_case.batch_fails,
        parameters=test_case.parameters,
        preexisting=test_case.preexisting,
    )

    assert recorded == test_case.expected_recorded
    assert failures == test_case.expected_failures
    assert tuple(log) == test_case.expected_log
    assert tuple(files) == test_case.expected_files
