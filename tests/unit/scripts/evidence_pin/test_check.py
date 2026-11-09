"""The evidence pin check against throwaway evidence and SQLQuack repositories."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.evidence_pin.main.check import run_pin_check
from tests.unit.scripts.evidence_pin._test_types import (
    EvidenceRepositories,
    PinCheckTestCase,
    PinMoveTestCase,
)
from tests.unit.scripts.evidence_pin.helpers import (
    append_receipt,
    delete_receipt,
    edit_case,
    evidence_repositories,
    leave_placeholder,
    malform_commit,
    pin_check_after,
    pin_main,
    pin_side_branch,
    pin_unfetched_offline,
    point_at,
    replace_history,
    rewrite_inputs,
    rewrite_receipt,
    tamper_with_snapshot,
)


@pytest.mark.parametrize(
    "test_case",
    (
        PinCheckTestCase(
            description="the placeholder pin passes, saying nothing is pinned",
            prepare=leave_placeholder,
            expected_return_code=0,
            expected_fragment="evidence pin: no evidence pinned yet",
        ),
        PinCheckTestCase(
            description="a verified pin on main passes",
            prepare=pin_main,
            expected_return_code=0,
            expected_fragment="evidence pin: passed, commit ",
        ),
        PinCheckTestCase(
            description="a commit that is not a full SHA fails",
            prepare=malform_commit,
            expected_return_code=1,
            expected_fragment="evidence pin: evidence.toml's commit is not a 40-digit lower-case hex SHA",
        ),
        PinCheckTestCase(
            description="a tampered snapshot fails on its content hash",
            prepare=tamper_with_snapshot,
            expected_return_code=1,
            expected_fragment="content hash mismatch",
        ),
        PinCheckTestCase(
            description="a pinned commit that cannot be fetched fails with the script's message",
            prepare=pin_unfetched_offline,
            expected_return_code=1,
            expected_fragment="(pinned in evidence.toml) is not in",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_pin_when_checking_then_reports_on_one_line(
    test_case: PinCheckTestCase,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    repositories: EvidenceRepositories = evidence_repositories(tmp_path)
    test_case.prepare(repositories)
    point_at(repositories, monkeypatch)

    return_code: int = run_pin_check(["--repository", str(repositories.root)])

    error: str = capsys.readouterr().err
    assert return_code == test_case.expected_return_code
    assert test_case.expected_fragment in error
    assert len(error.splitlines()) == 1


@pytest.mark.parametrize(
    "test_case",
    (
        PinMoveTestCase(
            description="appending to receipts and adding results passes",
            change=append_receipt,
            expected_return_code=0,
            expected_fragment="evidence pin: passed",
        ),
        PinMoveTestCase(
            description="rewriting a case passes",
            change=edit_case,
            expected_return_code=0,
            expected_fragment="evidence pin: passed",
        ),
        PinMoveTestCase(
            description="rewriting a receipts file fails",
            change=rewrite_receipt,
            expected_return_code=1,
            expected_fragment="syntax/receipts/stored.jsonl: recorded evidence is append-only, but it was rewritten",
        ),
        PinMoveTestCase(
            description="deleting a receipts file fails",
            change=delete_receipt,
            expected_return_code=1,
            expected_fragment="syntax/receipts/stored.jsonl: recorded evidence is append-only, but it was deleted",
        ),
        PinMoveTestCase(
            description="rewriting an inputs.jsonl fails",
            change=rewrite_inputs,
            expected_return_code=1,
            expected_fragment="translation/inputs.jsonl: recorded evidence is append-only, but it was rewritten",
        ),
        PinMoveTestCase(
            description="a new pin that does not descend from the old one fails",
            change=replace_history,
            expected_return_code=1,
            expected_fragment="the previous pin: pins only move forward",
        ),
        PinMoveTestCase(
            description="a new pin off the evidence repository's main fails",
            change=pin_side_branch,
            expected_return_code=1,
            expected_fragment="is not reachable from the evidence repository's main",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_pin_move_when_checking_then_only_forward_appends_pass(
    test_case: PinMoveTestCase,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    repositories: EvidenceRepositories = evidence_repositories(tmp_path)

    return_code: int = pin_check_after(
        repositories=repositories, change=test_case.change, monkeypatch=monkeypatch
    )

    error: str = capsys.readouterr().err
    assert return_code == test_case.expected_return_code
    assert test_case.expected_fragment in error
    assert len(error.splitlines()) == 1
