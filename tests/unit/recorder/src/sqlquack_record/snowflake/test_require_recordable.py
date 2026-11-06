"""Which warehouse errors are case outcomes, recorded, and which stop the run."""

from __future__ import annotations

import pytest
from snowflake.connector.errors import ProgrammingError

from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.snowflake._helpers.cursors import require_recordable
from tests.unit.recorder.src.sqlquack_record.snowflake._test_types import RecordableErrorTestCase


@pytest.mark.parametrize(
    "test_case",
    (
        RecordableErrorTestCase(
            description="a data exception is a case outcome",
            errno=100038,
            sqlstate="22018",
            expected_recordable=True,
        ),
        RecordableErrorTestCase(
            description="a Python function's handler error is a case outcome",
            errno=100357,
            sqlstate="P0000",
            expected_recordable=True,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_case_outcome_error_when_checking_recordable_then_it_passes(
    test_case: RecordableErrorTestCase,
) -> None:
    error: ProgrammingError = ProgrammingError(
        msg="failed", errno=test_case.errno, sqlstate=test_case.sqlstate
    )
    require_recordable(error)
    assert test_case.expected_recordable, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        RecordableErrorTestCase(
            description="the multi-statement wrapper error is not a case outcome",
            errno=100132,
            sqlstate="P0000",
            expected_recordable=False,
        ),
        RecordableErrorTestCase(
            description="a connection failure is not a case outcome",
            errno=390114,
            sqlstate="08001",
            expected_recordable=False,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_session_or_wrapper_error_when_checking_recordable_then_it_stops_the_run(
    test_case: RecordableErrorTestCase,
) -> None:
    error: ProgrammingError = ProgrammingError(
        msg="failed", errno=test_case.errno, sqlstate=test_case.sqlstate
    )
    with pytest.raises(RecorderError) as raised:
        require_recordable(error)
    assert "is not a case outcome" in str(raised.value), test_case.description
    assert not test_case.expected_recordable, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        RecordableErrorTestCase(
            description="a listed built-in function error is the outcome of a case expecting an error",
            errno=100344,
            sqlstate="P0000",
            expected_recordable=True,
            expected_error=True,
        ),
        RecordableErrorTestCase(
            description="the same error stops a case expecting success",
            errno=100344,
            sqlstate="P0000",
            expected_recordable=False,
        ),
        RecordableErrorTestCase(
            description="an unlisted P0000 error stops even a case expecting an error",
            errno=100999,
            sqlstate="P0000",
            expected_recordable=False,
            expected_error=True,
        ),
        RecordableErrorTestCase(
            description="a session failure stops a case expecting an error",
            errno=390114,
            sqlstate="08001",
            expected_recordable=False,
            expected_error=True,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_p0000_error_when_checking_recordable_then_only_listed_expected_errors_pass(
    test_case: RecordableErrorTestCase,
) -> None:
    error: ProgrammingError = ProgrammingError(
        msg="failed", errno=test_case.errno, sqlstate=test_case.sqlstate
    )
    if test_case.expected_recordable:
        require_recordable(error, expected_error=test_case.expected_error)
        return
    with pytest.raises(RecorderError, match="is not a case outcome"):
        require_recordable(error, expected_error=test_case.expected_error)
