"""The append-only evidence guard against real Git histories."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.unit.scripts.evidence_guard._test_types import GuardTestCase
from tests.unit.scripts.evidence_guard.helpers import (
    add_audit_valid,
    add_dialect_receipts,
    add_dialect_receipts_not_jsonl,
    add_error_message,
    add_error_number_message,
    add_mismatched,
    add_pinned,
    add_raw_parameters_hash,
    add_raw_query_id,
    add_referee_accepted,
    add_referee_catalogue,
    add_referee_rejected,
    add_referee_rejected_without_error,
    add_stand_in,
    add_syntax_receipts,
    add_syntax_receipts_not_jsonl,
    add_syntax_receipts_not_objects,
    add_syntax_receipts_with_report,
    add_syntax_report_not_object,
    add_types,
    add_types_with_arrow,
    add_valid,
    append_syntax_receipt,
    delete_dialect_receipts,
    delete_stored,
    edit_audit,
    edit_dialect_receipts,
    edit_referee,
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
            description="a new re-recorded audit result passes",
            change=add_audit_valid,
            expected_return_code=0,
            expected_fragment="passed, 2 added",
        ),
        GuardTestCase(
            description="an edited re-recorded audit result is rejected",
            change=edit_audit,
            expected_return_code=1,
            expected_fragment="append-only (status M)",
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
            description="a new result with its effective session parameters passes",
            change=add_pinned,
            expected_return_code=0,
            expected_fragment="passed, 2 added",
        ),
        GuardTestCase(
            description="a parameter snapshot that is not a digest is rejected",
            change=add_raw_parameters_hash,
            expected_return_code=1,
            expected_fragment="parameters_sha256 must be a SHA-256 hex digest",
        ),
        GuardTestCase(
            description="a raw query ID is rejected",
            change=add_raw_query_id,
            expected_return_code=1,
            expected_fragment="query_id_sha256 must be a SHA-256 hex digest",
        ),
        GuardTestCase(
            description="an error keeping its text message passes",
            change=add_error_message,
            expected_return_code=0,
            expected_fragment="passed, 2 added",
        ),
        GuardTestCase(
            description="an error message that is not text is rejected",
            change=add_error_number_message,
            expected_return_code=1,
            expected_fragment="an error records errno, sqlstate and at most a text message",
        ),
        GuardTestCase(
            description="a typing case's types-only result passes",
            change=add_types,
            expected_return_code=0,
            expected_fragment="passed, 1 added",
        ),
        GuardTestCase(
            description="a types outcome with an Arrow file is rejected",
            change=add_types_with_arrow,
            expected_return_code=1,
            expected_fragment="a types outcome keeps no Arrow file",
        ),
        GuardTestCase(
            description="a result not written by the recorder is rejected",
            change=add_stand_in,
            expected_return_code=1,
            expected_fragment="not written by sqlquack-record",
        ),
        GuardTestCase(
            description="a new accepted referee outcome passes",
            change=add_referee_accepted,
            expected_return_code=0,
            expected_fragment="passed, 1 added",
        ),
        GuardTestCase(
            description="a new rejected referee outcome with its error passes",
            change=add_referee_rejected,
            expected_return_code=0,
            expected_fragment="passed, 1 added",
        ),
        GuardTestCase(
            description="a rejected referee outcome without its error is rejected",
            change=add_referee_rejected_without_error,
            expected_return_code=1,
            expected_fragment="a rejected outcome records its error",
        ),
        GuardTestCase(
            description="an edited referee outcome is rejected",
            change=edit_referee,
            expected_return_code=1,
            expected_fragment="append-only",
        ),
        GuardTestCase(
            description="a function catalogue named by its hash passes",
            change=add_referee_catalogue,
            expected_return_code=0,
            expected_fragment="passed, 1 added",
        ),
        GuardTestCase(
            description="a new file of syntax receipts passes",
            change=add_syntax_receipts,
            expected_return_code=0,
            expected_fragment="passed, 1 added",
        ),
        GuardTestCase(
            description="receipts with the importer's run report beside them pass",
            change=add_syntax_receipts_with_report,
            expected_return_code=0,
            expected_fragment="passed, 2 added",
        ),
        GuardTestCase(
            description="a run report that is not a JSON object is rejected",
            change=add_syntax_report_not_object,
            expected_return_code=1,
            expected_fragment="new.report.json: not a JSON object",
        ),
        GuardTestCase(
            description="a receipt appended to a committed receipts file is rejected",
            change=append_syntax_receipt,
            expected_return_code=1,
            expected_fragment="append-only (status M)",
        ),
        GuardTestCase(
            description="syntax receipts outside JSON Lines are rejected",
            change=add_syntax_receipts_not_jsonl,
            expected_return_code=1,
            expected_fragment="syntax receipts are .jsonl files",
        ),
        GuardTestCase(
            description="a receipts line that is not an object is rejected",
            change=add_syntax_receipts_not_objects,
            expected_return_code=1,
            expected_fragment="new.jsonl:2: not a JSON object",
        ),
        GuardTestCase(
            description="a new file of a dialect's receipts passes",
            change=add_dialect_receipts,
            expected_return_code=0,
            expected_fragment="passed, 1 added",
        ),
        GuardTestCase(
            description="a deleted dialect receipts file is rejected",
            change=delete_dialect_receipts,
            expected_return_code=1,
            expected_fragment="syntax-postgres/receipts/stored.jsonl: recorded results are append-only (status D)",
        ),
        GuardTestCase(
            description="a rewritten dialect receipts file is rejected",
            change=edit_dialect_receipts,
            expected_return_code=1,
            expected_fragment="append-only (status M)",
        ),
        GuardTestCase(
            description="a dialect's receipts outside JSON Lines are rejected",
            change=add_dialect_receipts_not_jsonl,
            expected_return_code=1,
            expected_fragment="syntax receipts are .jsonl files",
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
