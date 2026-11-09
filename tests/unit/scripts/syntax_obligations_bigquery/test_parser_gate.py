"""The parser gate on recorded verdicts: a case the pinned parser contradicts stops the build."""

from __future__ import annotations

import pytest

from scripts.grammar_obligations.exceptions import ObligationError
from scripts.grammar_obligations.types import Row
from scripts.syntax_obligations_bigquery._helpers.parser import (
    disagreements,
    engine_confirmed,
    parser_text,
    refuse_refused_withdrawals,
    refuse_unparsed_shapes,
    settle_observed,
    unmapped_refusals,
)
from scripts.syntax_obligations_bigquery.models import Verdict
from tests.unit.scripts.syntax_obligations_bigquery._test_types import (
    GateTestCase,
    SettleTestCase,
    ShapeTestCase,
    TokensTestCase,
    WithdrawalTestCase,
)

SQL: str = "SELECT * FROM SQLQUACK_PROJECT.SQLQUACK_DATASET.t1"


@pytest.mark.parametrize(
    "test_case",
    (
        GateTestCase(
            description="an accept case that parses",
            expect="accept",
            stage="",
            parsed=True,
            message="",
            expected_disagreements=0,
            expected_unmapped=[],
        ),
        GateTestCase(
            description="an accept case the parser refuses",
            expect="accept",
            stage="",
            parsed=False,
            message="Syntax error: Unexpected end of statement [at 1:9]",
            expected_disagreements=1,
            expected_unmapped=[],
        ),
        GateTestCase(
            description="an accept case refused only for the open build's object type",
            expect="accept",
            stage="",
            parsed=False,
            message="RESERVATION is not a supported object type [at 1:8]",
            expected_disagreements=0,
            expected_unmapped=[],
        ),
        GateTestCase(
            description="a reject case that parses",
            expect="reject",
            stage="",
            parsed=True,
            message="",
            expected_disagreements=1,
            expected_unmapped=[],
        ),
        GateTestCase(
            description="a reject case expected at analysis, which must parse",
            expect="reject",
            stage="semantic",
            parsed=True,
            message="",
            expected_disagreements=0,
            expected_unmapped=[],
        ),
        GateTestCase(
            description="a reject case that parses, which the engine refused at syntax",
            expect="reject",
            stage="",
            parsed=True,
            message="",
            expected_disagreements=0,
            expected_unmapped=[],
            receipt_phase="syntax_error",
        ),
        GateTestCase(
            description="a reject case that parses, which the engine refused at analysis",
            expect="reject",
            stage="",
            parsed=True,
            message="",
            expected_disagreements=1,
            expected_unmapped=[],
            receipt_phase="semantic_error",
        ),
        GateTestCase(
            description="an accept case the parser refuses, which the engine accepted",
            expect="accept",
            stage="",
            parsed=False,
            message="Syntax error: Unexpected end of statement [at 1:9]",
            expected_disagreements=0,
            expected_unmapped=[],
            receipt_phase="accepted",
        ),
        GateTestCase(
            description="a reject case refused with a message no class maps",
            expect="reject",
            stage="",
            parsed=False,
            message="Something the grammar never says",
            expected_disagreements=0,
            expected_unmapped=["Something the grammar never says"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_recorded_verdict_when_gated_then_contradictions_and_unmapped_refusals_show(
    test_case: GateTestCase,
) -> None:
    case: Row = {
        "case_id": "bq.x.o001",
        "expect": test_case.expect,
        "stage": test_case.stage,
        "sql": SQL,
        "sql_sha256": "h",
    }
    found: dict[str, Verdict] = {SQL: Verdict(parsed=test_case.parsed, message=test_case.message)}
    phases: dict[tuple[str, str], str] = {("bq.x.o001", "h"): test_case.receipt_phase}
    confirmed: frozenset[str] = engine_confirmed(cases=[case], phases=phases)

    wrong: list[str] = disagreements(cases=[case], found=found, confirmed=confirmed)
    unmapped: list[str] = unmapped_refusals(cases=[case], found=found)

    assert len(wrong) == test_case.expected_disagreements
    assert unmapped == test_case.expected_unmapped


@pytest.mark.parametrize(
    "test_case",
    (
        TokensTestCase(
            description="a project and dataset path",
            sql=SQL,
            expected_sql="SELECT * FROM project1.dataset1.t1",
        ),
        TokensTestCase(
            description="the second dataset in backquotes, and a longer word left alone",
            sql="SELECT * FROM `SQLQUACK_DATASET2.t2` , SQLQUACK_DATASETS",
            expected_sql="SELECT * FROM `dataset2.t2` , SQLQUACK_DATASETS",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_case_tokens_when_read_for_the_parser_then_they_become_plain_names(
    test_case: TokensTestCase,
) -> None:
    assert parser_text(test_case.sql) == test_case.expected_sql


@pytest.mark.parametrize(
    "test_case",
    (
        WithdrawalTestCase(
            description="a withdrawn text the parser refuses",
            sql="SELECT ML . ( v1 ) OVER ( )",
            parsed=False,
            expected_error="1 suppressed texts the pinned parser refuses",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_documented_suppression_when_gating_then_a_refused_text_stops_the_build(
    test_case: WithdrawalTestCase,
) -> None:
    found: dict[str, Verdict] = {test_case.sql: Verdict(parsed=test_case.parsed, message="m")}
    withdrawn: list[tuple[str, str]] = [("bq.ddl.f", test_case.sql)]

    with pytest.raises(ObligationError, match=test_case.expected_error):
        refuse_refused_withdrawals(withdrawn=withdrawn, found=found)


@pytest.mark.parametrize(
    "test_case",
    (
        SettleTestCase(
            description="an observed suppressed text the parser takes",
            parsed=True,
            settle=frozenset({SQL}),
            expected_expect="observe",
        ),
        SettleTestCase(
            description="an observed suppressed text the parser refuses",
            parsed=False,
            settle=frozenset({SQL}),
            expected_expect="reject",
        ),
        SettleTestCase(
            description="an observed suppressed text refused only for its object type",
            parsed=False,
            settle=frozenset({SQL}),
            expected_expect="observe",
            message="RESERVATION is not a supported object type [at 1:8]",
        ),
        SettleTestCase(
            description="an observed text no suppression made observed",
            parsed=False,
            settle=frozenset(),
            expected_expect="observe",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_observed_case_when_gating_then_only_a_refused_suppressed_one_is_a_reject(
    test_case: SettleTestCase,
) -> None:
    cases: list[Row] = [{"case_id": "a", "expect": "observe", "sql": SQL}]
    found: dict[str, Verdict] = {SQL: Verdict(parsed=test_case.parsed, message=test_case.message)}

    settle_observed(cases=cases, settle=test_case.settle, found=found)

    assert cases[0]["expect"] == test_case.expected_expect


@pytest.mark.parametrize(
    "test_case",
    (
        ShapeTestCase(
            description="a SHAPE receipt whose text parses",
            parsed=True,
            message="",
            expected_error="",
        ),
        ShapeTestCase(
            description="a SHAPE receipt whose text the parser refuses at syntax",
            parsed=False,
            message="Syntax error: Unexpected end of statement [at 1:9]",
            expected_error="1 SHAPE receipts the pinned parser refuses",
        ),
        ShapeTestCase(
            description="a SHAPE receipt refused only for its object type",
            parsed=False,
            message="RESERVATION is not a supported object type [at 1:8]",
            expected_error="",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_shape_receipt_when_gating_then_its_text_must_parse(
    test_case: ShapeTestCase,
) -> None:
    case: Row = {"case_id": "bq.x.k1", "expect": "accept", "sql": SQL, "sql_sha256": "h"}
    found: dict[str, Verdict] = {SQL: Verdict(parsed=test_case.parsed, message=test_case.message)}

    raised: str = ""
    try:
        refuse_unparsed_shapes(cases=[case], found=found, shaped=frozenset({"h"}))
    except ObligationError as error:
        raised = str(error)

    assert raised.startswith(test_case.expected_error)
