"""The append-only evidence guard against real Git histories."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.unit.scripts.evidence_guard._test_types import GuardTestCase
from tests.unit.scripts.evidence_guard.helpers import (
    add_error_message,
    add_mismatched,
    add_raw_query_id,
    add_stand_in,
    add_valid,
    delete_stored,
    edit_stored,
    guard_after,
    unchanged,
)


@pytest.mark.parametrize(
    "test_case",
    (
        GuardTestCase(
            description="an unrelated change passes",
            change=unchanged,
            expected_return_code=0,
            expected_fragment="passed, 0 added",
        ),
        GuardTestCase(
            description="a new recorded result passes",
            change=add_valid,
            expected_return_code=0,
            expected_fragment="passed, 2 added",
        ),
        GuardTestCase(
            description="an edited result is rejected",
            change=edit_stored,
            expected_return_code=1,
            expected_fragment="append-only (status M)",
        ),
        GuardTestCase(
            description="a deleted result is rejected",
            change=delete_stored,
            expected_return_code=1,
            expected_fragment="append-only (status D)",
        ),
        GuardTestCase(
            description="a new result whose Arrow file does not match its metadata is rejected",
            change=add_mismatched,
            expected_return_code=1,
            expected_fragment="does not match arrow_sha256",
        ),
        GuardTestCase(
            description="a raw query ID is rejected",
            change=add_raw_query_id,
            expected_return_code=1,
            expected_fragment="query_id_sha256 must be a SHA-256 hex digest",
        ),
        GuardTestCase(
            description="an error message is rejected",
            change=add_error_message,
            expected_return_code=1,
            expected_fragment="an error records only errno and sqlstate",
        ),
        GuardTestCase(
            description="a result not written by the recorder is rejected",
            change=add_stand_in,
            expected_return_code=1,
            expected_fragment="not written by sqlquack-record",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_result_change_when_guarding_then_only_valid_additions_pass(
    test_case: GuardTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    return_code: int = guard_after(repository=tmp_path, change=test_case.change)

    assert return_code == test_case.expected_return_code
    assert test_case.expected_fragment in capsys.readouterr().err
