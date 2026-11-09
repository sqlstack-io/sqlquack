"""A prepare error is a syntax error only when the parser itself raised it (see the README)."""

from __future__ import annotations

import pytest

from scripts.sqlite_receipts._helpers.judge import prepare_phase
from scripts.sqlite_receipts.exceptions import RefereeError
from tests.unit.scripts.sqlite_receipts._test_types import PhaseTestCase, UnreviewedPhaseTestCase


@pytest.mark.parametrize(
    "test_case",
    (
        PhaseTestCase(
            description="no move for a token",
            code=1,
            message='near "FROM": syntax error',
            expected_code="SYNTX",
            expected_phase="syntax_error",
        ),
        PhaseTestCase(
            description="the text ends inside a statement",
            code=1,
            message="incomplete input",
            expected_code="INCMP",
            expected_phase="syntax_error",
        ),
        PhaseTestCase(
            description="the tokenizer refuses a character",
            code=1,
            message="unrecognized token: \"X'0'\"",
            expected_code="UNTOK",
            expected_phase="syntax_error",
        ),
        PhaseTestCase(
            description="a parse.y action refuses a table option",
            code=1,
            message="unknown table option: ROWID",
            expected_code="GRAMR",
            expected_phase="syntax_error",
        ),
        PhaseTestCase(
            description="a parse.y action refuses a join keyword",
            code=1,
            message="unknown join type: OUTER",
            expected_code="GRAMR",
            expected_phase="syntax_error",
        ),
        PhaseTestCase(
            description="a build.c helper refuses, and the statement alone refuses the same",
            code=1,
            message="1 columns assigned 2 values",
            statement="UPDATE t1 SET ( c1 ) = ( 1 , 2 )",
            alone="1 columns assigned 2 values",
            expected_code="GRAMR",
            expected_phase="syntax_error",
        ),
        PhaseTestCase(
            description="the same message, which the statement alone does not raise",
            code=1,
            message="1 columns assigned 2 values",
            statement="UPDATE t1 SET ( c1 ) = ( SELECT * FROM t1 )",
            alone="no such table: t1",
            expected_code="E0001",
            expected_phase="semantic_error",
        ),
        PhaseTestCase(
            description="CREATE TABLE names a column it does not define",
            code=1,
            message="no such column: c9",
            statement='CREATE TEMP TABLE IF NOT EXISTS "temp" . t3 ( c1 CHECK ( c9 ) )',
            alone="no such column: c9",
            expected_code="GRAMR",
            expected_phase="syntax_error",
        ),
        PhaseTestCase(
            description="CREATE TABLE AS SELECT resolves a name, so no column list",
            code=1,
            message="no such column: c9",
            statement="CREATE TABLE t3 AS SELECT c9",
            alone="no such column: c9",
            expected_code="E0001",
            expected_phase="semantic_error",
        ),
        PhaseTestCase(
            description="a name binding refuses",
            code=1,
            message="no such table: t9",
            expected_code="E0001",
            expected_phase="semantic_error",
        ),
        PhaseTestCase(
            description="a quoted string that names syntax is still binding",
            code=1,
            message='no such column: "near \\"x\\": syntax error"',
            expected_code="E0001",
            expected_phase="semantic_error",
        ),
        PhaseTestCase(
            description="a missing collation, its extended code kept",
            code=257,
            message="no such collation sequence: c9",
            expected_code="E0257",
            expected_phase="semantic_error",
        ),
        PhaseTestCase(
            description="a documented function the build leaves out",
            code=1,
            message="no such function: SOUNDEX",
            expected_code="NOFTR",
            expected_phase="unsupported_feature",
        ),
        PhaseTestCase(
            description="a PRAGMA that acts while it is prepared",
            code=1,
            message="Safety level may not be changed inside a transaction",
            expected_code="E0001",
            expected_phase="runtime_error",
        ),
        PhaseTestCase(
            description="memory, not the text",
            code=7,
            message="out of memory",
            expected_code="E0007",
            expected_phase="runtime_error",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_prepare_error_when_mapped_then_only_the_parser_s_own_is_syntax(
    test_case: PhaseTestCase,
) -> None:
    code, phase = prepare_phase(
        code=test_case.code,
        message=test_case.message,
        statement=test_case.statement,
        alone=lambda: test_case.alone,
    )

    assert (code, phase) == (test_case.expected_code, test_case.expected_phase)


@pytest.mark.parametrize(
    "test_case",
    (
        UnreviewedPhaseTestCase(
            description="a message no class reviews",
            code=1,
            message="a message no class reviews",
            expected_error="no reviewed class",
        ),
        UnreviewedPhaseTestCase(
            description="an empty message", code=1, message="", expected_error="no reviewed class"
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_prepare_error_in_no_reviewed_class_when_mapped_then_the_run_stops(
    test_case: UnreviewedPhaseTestCase,
) -> None:
    with pytest.raises(RefereeError, match=test_case.expected_error):
        prepare_phase(
            code=test_case.code,
            message=test_case.message,
            statement=test_case.statement,
            alone=lambda: test_case.alone,
        )
