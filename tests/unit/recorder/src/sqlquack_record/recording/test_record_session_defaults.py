"""Recording the account's default values of the parameters cases set or pin."""

from __future__ import annotations

from pathlib import Path

import pytest

from sqlquack_record.recording.exceptions import RecorderError
from tests.unit.recorder.src.sqlquack_record.recording._test_types import DefaultsTestCase
from tests.unit.recorder.src.sqlquack_record.recording.helpers import session_defaults


@pytest.mark.parametrize(
    "test_case",
    (
        DefaultsTestCase(
            description="only the named parameters are written, with the snapshot hash",
            names=frozenset({"TIMEZONE"}),
            expected_parameters={"TIMEZONE": "UTC"},
            expected_error="",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_named_parameters_when_recording_defaults_then_only_they_are_written(
    test_case: DefaultsTestCase, tmp_path: Path
) -> None:
    """Given named parameters, when recording defaults, then only they are written."""
    document: dict[str, object] = session_defaults(directory=tmp_path, names=test_case.names)
    assert (document["parameters"], len(str(document["parameters_sha256"]))) == (
        test_case.expected_parameters,
        64,
    )


@pytest.mark.parametrize(
    "test_case",
    (
        DefaultsTestCase(
            description="a parameter the session does not report fails the recording",
            names=frozenset({"WEEK_START"}),
            expected_parameters={},
            expected_error="no value for WEEK_START",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_unreported_parameter_when_recording_defaults_then_it_fails(
    test_case: DefaultsTestCase, tmp_path: Path
) -> None:
    """Given an unreported parameter, when recording defaults, then the recorder fails."""
    with pytest.raises(RecorderError, match=test_case.expected_error):
        session_defaults(directory=tmp_path, names=test_case.names)
