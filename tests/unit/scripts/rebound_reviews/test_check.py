"""The re-bound review check refuses every change but the one its class allows."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.rebound_reviews.main.check import run_rebound_check
from tests.unit.scripts.rebound_reviews._test_types import ReboundTestCase, StageReboundTestCase
from tests.unit.scripts.rebound_reviews.helpers import call, write_inputs, write_stage_inputs

_SQL: str = "SELECT COUNT(c1) FROM t1"
_FALSE: str = call('"distinct":false')


@pytest.mark.parametrize(
    "test_case",
    (
        ReboundTestCase(
            description="a false flag read as none passes",
            sql=_SQL,
            old_tree=_FALSE,
            new_tree=call('"set_quantifier":"none"'),
            reviewed_tree=_FALSE,
            expected_code=0,
            expected_fragment="1 of 1 forms in 1 reviews diff-checked; passed",
        ),
        ReboundTestCase(
            description="a true flag read as distinct passes",
            sql="SELECT COUNT(DISTINCT c1) FROM t1",
            old_tree=call('"distinct":true'),
            new_tree=call('"set_quantifier":"distinct"'),
            reviewed_tree=call('"distinct":true'),
            expected_code=0,
            expected_fragment="passed",
        ),
        ReboundTestCase(
            description="all passes where the SQL writes ( ALL",
            sql="SELECT COUNT( all c1) FROM t1",
            old_tree=_FALSE,
            new_tree=call('"set_quantifier":"all"'),
            reviewed_tree=_FALSE,
            expected_code=0,
            expected_fragment="passed",
        ),
        ReboundTestCase(
            description="all is refused where the SQL writes no ( ALL, even with UNION ALL",
            sql="SELECT COUNT(c1) FROM t1 UNION ALL SELECT 1",
            old_tree=_FALSE,
            new_tree=call('"set_quantifier":"all"'),
            reviewed_tree=_FALSE,
            expected_code=1,
            expected_fragment="1 calls read as all, but the SQL writes ( ALL 0 times",
        ),
        ReboundTestCase(
            description="a true flag read as none is refused",
            sql=_SQL,
            old_tree=call('"distinct":true'),
            new_tree=call('"set_quantifier":"none"'),
            reviewed_tree=call('"distinct":true'),
            expected_code=1,
            expected_fragment="distinct True became set_quantifier 'none'",
        ),
        ReboundTestCase(
            description="any other change alongside the rename is refused",
            sql=_SQL,
            old_tree=_FALSE,
            new_tree=call('"set_quantifier":"none"').replace('"c1"', '"c2"'),
            reviewed_tree=_FALSE,
            expected_code=1,
            expected_fragment="'c1' became 'c2'",
        ),
        ReboundTestCase(
            description="a moved field is refused",
            sql=_SQL,
            old_tree=_FALSE,
            new_tree=call('"set_quantifier":"none"').replace(
                '"name":"COUNT","set_quantifier":"none"', '"set_quantifier":"none","name":"COUNT"'
            ),
            reviewed_tree=_FALSE,
            expected_code=1,
            expected_fragment="fields",
        ),
        ReboundTestCase(
            description="old trees other than the reviewed ones are refused",
            sql=_SQL,
            old_tree=_FALSE,
            new_tree=call('"set_quantifier":"none"'),
            reviewed_tree=call('"distinct":true'),
            expected_code=1,
            expected_fragment="the old trees are not the ones the earlier review approved",
        ),
        ReboundTestCase(
            description="no re-bound review fails closed",
            sql=_SQL,
            old_tree=_FALSE,
            new_tree=_FALSE,
            reviewed_tree=_FALSE,
            expected_code=1,
            expected_fragment="0 of 0 forms in 0 reviews diff-checked; FAILED",
            note_field="comment",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_rebound_review_when_checked_then_only_the_quantifier_rename_passes(
    test_case: ReboundTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    arguments: list[str] = write_inputs(
        tmp_path,
        sql=test_case.sql,
        old_tree=test_case.old_tree,
        new_tree=test_case.new_tree,
        reviewed_tree=test_case.reviewed_tree,
        note_field=test_case.note_field,
    )

    code: int = run_rebound_check(arguments)

    assert code == test_case.expected_code
    assert test_case.expected_fragment in capsys.readouterr().err


_ACCEPT: dict[str, object] = {"expect": "accept"}


@pytest.mark.parametrize(
    "test_case",
    (
        StageReboundTestCase(
            description="a stage added that is its receipt's phase passes",
            phase="semantic_error",
            reviewed_stage=None,
            old_case=_ACCEPT,
            new_case={**_ACCEPT, "stage": "semantic"},
            expected_code=0,
            expected_fragment="1 of 1 forms in 1 reviews diff-checked; passed",
        ),
        StageReboundTestCase(
            description="a stage added that is not its receipt's phase is refused",
            phase="semantic_error",
            reviewed_stage=None,
            old_case=_ACCEPT,
            new_case={**_ACCEPT, "stage": "runtime"},
            expected_code=1,
            expected_fragment="stage 'runtime' is not the stage of its receipt's semantic_error",
        ),
        StageReboundTestCase(
            description="a stage dropped from a case whose receipt accepted it passes",
            phase="accepted",
            reviewed_stage="semantic",
            old_case={**_ACCEPT, "stage": "semantic"},
            new_case=_ACCEPT,
            expected_code=0,
            expected_fragment="passed",
        ),
        StageReboundTestCase(
            description="a stage dropped under a syntax error phase is refused",
            phase="syntax_error",
            reviewed_stage="semantic",
            old_case={**_ACCEPT, "stage": "semantic"},
            new_case=_ACCEPT,
            expected_code=1,
            expected_fragment="'syntax_error' is no receipt phase that gives a stage",
        ),
        StageReboundTestCase(
            description="a stage dropped under an unknown phase is refused",
            phase="some_new_phase",
            reviewed_stage="semantic",
            old_case={**_ACCEPT, "stage": "semantic"},
            new_case=_ACCEPT,
            expected_code=1,
            expected_fragment="'some_new_phase' is no receipt phase that gives a stage",
        ),
        StageReboundTestCase(
            description="a stage dropped with no receipt phase is refused",
            phase=None,
            reviewed_stage="semantic",
            old_case={**_ACCEPT, "stage": "semantic"},
            new_case=_ACCEPT,
            expected_code=1,
            expected_fragment="None is no receipt phase that gives a stage",
        ),
        StageReboundTestCase(
            description="a stage changed on a case expected to be rejected is refused",
            phase="semantic_error",
            reviewed_stage=None,
            old_case={"expect": "reject"},
            new_case={"expect": "reject", "stage": "semantic"},
            expected_code=1,
            expected_fragment="it is not expected to be accepted",
        ),
        StageReboundTestCase(
            description="another field changed beside the stage is refused",
            phase="semantic_error",
            reviewed_stage=None,
            old_case={"expect": "observe"},
            new_case={**_ACCEPT, "stage": "semantic"},
            expected_code=1,
            expected_fragment="expect changed, not only its stage",
        ),
        StageReboundTestCase(
            description="old cases other than the reviewed ones are refused",
            phase="semantic_error",
            reviewed_stage="runtime",
            old_case=_ACCEPT,
            new_case={**_ACCEPT, "stage": "semantic"},
            expected_code=1,
            expected_fragment="the old cases are not the ones the earlier review approved",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_stage_rebound_review_when_checked_then_only_its_receipt_stage_passes(
    test_case: StageReboundTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    arguments: list[str] = write_stage_inputs(
        tmp_path,
        phase=test_case.phase,
        reviewed_stage=test_case.reviewed_stage,
        old_case=test_case.old_case,
        new_case=test_case.new_case,
    )

    code: int = run_rebound_check(arguments)

    assert code == test_case.expected_code
    assert test_case.expected_fragment in capsys.readouterr().err
