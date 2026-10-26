"""Recording pending cases against a fake warehouse session."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.unit.recorder.src.sqlquack_record.recording._test_types import (
    BrokenCaseTestCase,
    ParametersTestCase,
    RecordedFile,
    RecordTestCase,
)
from tests.unit.recorder.src.sqlquack_record.recording.helpers import (
    broken_case_message,
    record,
    recorded_parameters,
)

_UTC: tuple[tuple[str, bool | int | str], ...] = (("TIMEZONE", "UTC"),)
_WEEKS: tuple[tuple[str, bool | int | str], ...] = (("TIMEZONE", "UTC"), ("WEEK_START", 1))
_ROWS: RecordedFile = ("rows", True, True, True, "2")
_ERROR: RecordedFile = ("error", True, True, True, "null")
_EXISTING: RecordedFile = ("preexisting", False, True, True, "null")
_TYPES: RecordedFile = ("types", True, True, True, "2")


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
            without_values=frozenset(),
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
            without_values=frozenset(),
            expected_recorded=2,
            expected_failures=1,
            expected_log=("open", "batch", "one", "one", "close"),
            expected_files=(_ROWS, _ERROR),
        ),
        RecordTestCase(
            description="a case creating functions runs alone, then drops its functions",
            statements=("SELECT 1", "SELECT order_total(2)", "SELECT 22"),
            failing=frozenset(),
            batch_fails=False,
            parameters=(_UTC,),
            preexisting=(),
            without_values=frozenset(),
            with_functions=frozenset({"SELECT order_total(2)"}),
            expected_recorded=3,
            expected_failures=0,
            expected_log=(
                "open",
                "batch",
                "create function",
                "one",
                "drop function",
                "close",
            ),
            expected_files=(_ROWS, _ROWS, _ROWS),
        ),
        RecordTestCase(
            description="an existing result is never re-recorded",
            statements=("SELECT 1", "SELECT 22"),
            failing=frozenset(),
            batch_fails=False,
            parameters=(_UTC,),
            preexisting=("SELECT 1",),
            without_values=frozenset(),
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
            without_values=frozenset(),
            expected_recorded=2,
            expected_failures=0,
            expected_log=("open", "one", "close", "open", "one", "close"),
            expected_files=(_ROWS, _ROWS),
        ),
        RecordTestCase(
            description="a typing case without values stores its types and row count, no Arrow",
            statements=("SELECT 1", "SELECT 22"),
            failing=frozenset(),
            batch_fails=False,
            parameters=(_UTC,),
            preexisting=(),
            without_values=frozenset({"SELECT 22"}),
            expected_recorded=2,
            expected_failures=0,
            expected_log=("open", "batch", "close"),
            expected_files=(_ROWS, _TYPES),
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
        without_values=test_case.without_values,
        with_functions=test_case.with_functions,
    )

    assert recorded == test_case.expected_recorded
    assert failures == test_case.expected_failures
    assert tuple(log) == test_case.expected_log
    assert tuple(files) == test_case.expected_files


@pytest.mark.parametrize(
    "test_case",
    (
        BrokenCaseTestCase(
            description="a statement that does not compile stops the run and names its case",
            statements=("SELECT 1", "SELECT NO_SUCH_FUNCTION(1)", "SELECT 22"),
            broken="SELECT NO_SUCH_FUNCTION(1)",
            expected_mention="case fake.toml#26 (pending file",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_case_that_does_not_compile_when_recording_then_the_error_names_the_case(
    test_case: BrokenCaseTestCase, tmp_path: Path
) -> None:
    message: str = broken_case_message(
        directory=tmp_path, statements=test_case.statements, broken=test_case.broken
    )

    assert test_case.expected_mention in message


@pytest.mark.parametrize(
    "test_case",
    (
        ParametersTestCase(
            description="case and pinned parameters are stored with their reported values",
            parameters=(("TIMEZONE", "UTC"), ("WEEK_START", 0)),
            expected_effective={
                "DATE_OUTPUT_FORMAT": "YYYY-MM-DD",
                "TIMEZONE": "UTC",
                "WEEK_START": "0",
            },
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_pinned_parameters_when_recording_then_effective_values_are_stored(
    test_case: ParametersTestCase, tmp_path: Path
) -> None:
    assert recorded_parameters(directory=tmp_path, parameters=test_case.parameters) == (
        test_case.expected_effective,
        True,
    )
