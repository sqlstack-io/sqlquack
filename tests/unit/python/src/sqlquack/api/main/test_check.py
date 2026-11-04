"""`sqlquack.check` reports what `sqlquack check` reports."""

from __future__ import annotations

import pytest

import sqlquack
from sqlquack import CheckResult
from tests.unit.python.src.sqlquack.api.main._test_types import (
    CheckTestCase,
    FailureTestCase,
    RenderTestCase,
)
from tests.unit.python.src.sqlquack.api.main.helpers import positions


@pytest.mark.parametrize(
    "test_case",
    (
        CheckTestCase(
            description="sound SQL has no diagnostics",
            sql="SELECT order_id FROM orders",
            expected_ok=True,
            expected_codes=(),
            expected_positions=(),
        ),
        CheckTestCase(
            description="SQL that does not parse points where the parser stopped",
            sql="SELECT a,\n  FROM orders WHERE (b = 1\n",
            expected_ok=False,
            expected_codes=("SQ0001",),
            expected_positions=((2, 26),),
        ),
        CheckTestCase(
            description="each construct Snowflake rejects has its own code, in order",
            sql="SELECT 1 QUALIFY TRUE;\nSELECT 1 INTERSECT ALL SELECT 2",
            expected_ok=False,
            expected_codes=("SQ0203", "SQ0201"),
            expected_positions=((1, 1), (2, 1)),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_sql_when_checked_then_diagnostics_carry_codes_and_positions(
    test_case: CheckTestCase,
) -> None:
    result: CheckResult = sqlquack.check(sql=test_case.sql)
    assert result.ok is test_case.expected_ok
    assert tuple(found.code for found in result.diagnostics) == test_case.expected_codes
    assert positions(result) == test_case.expected_positions


@pytest.mark.parametrize(
    "test_case",
    (
        RenderTestCase(
            description="a rejection renders as the command line prints it",
            sql="SELECT 1 QUALIFY TRUE",
            path="query.sql",
            expected_rendered=(
                "error[SQ0203]: QUALIFY needs a window function\n --> query.sql:1:1\n  |\n"
                "1 | SELECT 1 QUALIFY TRUE\n  | ^^^^^^^^^^^^^^^^^^^^^ QUALIFY is used without a "
                "window function in the SELECT list or QUALIFY\n  |\n  = help: move the condition "
                "to WHERE or HAVING, or filter on a window function's result\n"
            ),
            expected_help=(
                "move the condition to WHERE or HAVING, or filter on a window function's result"
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_rejection_when_checked_then_it_renders_as_the_command_line_does(
    test_case: RenderTestCase,
) -> None:
    result: CheckResult = sqlquack.check(sql=test_case.sql, path=test_case.path)
    assert result.rendered == test_case.expected_rendered
    assert result.diagnostics[0].help == test_case.expected_help


@pytest.mark.parametrize(
    "test_case",
    (
        FailureTestCase(
            description="input of only whitespace",
            sql="   ",
            options=None,
            expected_message="no SQL statement",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_no_statement_when_checking_then_sqlquack_error_is_raised(
    test_case: FailureTestCase,
) -> None:
    with pytest.raises(sqlquack.SqlQuackError, match=test_case.expected_message):
        sqlquack.check(sql=test_case.sql)
