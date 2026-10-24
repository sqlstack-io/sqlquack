"""The append-only evidence guard against real Git histories."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.scripts.evidence_guard._test_types import GuardTestCase
from tests.scripts.evidence_guard.helpers import guard_after


@pytest.mark.parametrize(
    "test_case",
    (
        GuardTestCase(
            description="an unrelated change passes",
            change="none",
            expected_return_code=0,
            expected_fragment="passed, 0 added",
        ),
        GuardTestCase(
            description="a new recorded result passes",
            change="add",
            expected_return_code=0,
            expected_fragment="passed, 2 added",
        ),
        GuardTestCase(
            description="an edited result is rejected",
            change="edit",
            expected_return_code=1,
            expected_fragment="append-only (status M)",
        ),
        GuardTestCase(
            description="a deleted result is rejected",
            change="delete",
            expected_return_code=1,
            expected_fragment="append-only (status D)",
        ),
        GuardTestCase(
            description="a new result whose Arrow file does not match its metadata is rejected",
            change="add-mismatched",
            expected_return_code=1,
            expected_fragment="does not match arrow_sha256",
        ),
        GuardTestCase(
            description="a result not written by the recorder is rejected",
            change="add-stand-in",
            expected_return_code=1,
            expected_fragment="not written by sqlquack-record",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_result_change_when_guarding_then_only_valid_additions_pass(
    test_case: GuardTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    return_code: int = guard_after(tmp_path, test_case.change)

    assert return_code == test_case.expected_return_code
    assert test_case.expected_fragment in capsys.readouterr().err
