"""`sqlquack.check` reports what `sqlquack check` reports."""

from __future__ import annotations

import threading

import pytest

import sqlquack
from sqlquack import CheckResult
from tests.unit.python.src.sqlquack.api.main._test_types import (
    CallDepthRefusalTestCase,
    CallDepthTestCase,
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
            expected_positions=((1, 10), (2, 10)),
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
                "error[SQ0203]: QUALIFY needs a window function\n --> query.sql:1:10\n  |\n"
                "1 | SELECT 1 QUALIFY TRUE\n  |          ^^^^^^^ QUALIFY is used without a "
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


SMALL_STACK_BYTES: int = 256 * 1024
FIXED_NESTING_HELP: str = (
    "remove redundant parentheses, or move nested parts into common table expressions; "
    "the limit of 512 is fixed, and --max-call-depth can't raise it"
)


@pytest.mark.parametrize(
    "test_case",
    (
        CallDepthTestCase(
            description="256 nested calls are read by default",
            levels=256,
            max_call_depth=None,
            expected_codes=(),
            expected_helps=(),
            expected_labels=(),
        ),
        CallDepthTestCase(
            description="257 nested calls are refused, saying how to raise the limit",
            levels=257,
            max_call_depth=None,
            expected_codes=("SQ0107",),
            expected_helps=(
                "compute inner calls in a common table expression first, or raise the limit, up "
                "to 512, with --max-call-depth (command line) or max_call_depth= (Python)",
            ),
            expected_labels=("value 257 exceeds configured limit 256",),
        ),
        CallDepthTestCase(
            description="max_call_depth=300 reads 300 nested calls",
            levels=300,
            max_call_depth=300,
            expected_codes=(),
            expected_helps=(),
            expected_labels=(),
        ),
        CallDepthTestCase(
            description="at the ceiling, 600 nested calls meet the fixed limit, which says so",
            levels=600,
            max_call_depth=512,
            expected_codes=("SQ0103",),
            expected_helps=(FIXED_NESTING_HELP,),
            expected_labels=(
                "value 513 exceeds configured limit 512; 512 is SQLQuack's fixed nesting ceiling, "
                "which no setting raises",
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_nested_calls_when_checked_on_a_small_stack_then_the_call_depth_limit_decides(
    test_case: CallDepthTestCase,
) -> None:
    sql: str = "SELECT " + "ABS(" * test_case.levels + "1" + ")" * test_case.levels
    results: list[CheckResult] = []
    previous: int = threading.stack_size(SMALL_STACK_BYTES)
    try:
        thread: threading.Thread = threading.Thread(
            target=lambda: results.append(
                sqlquack.check(sql=sql, max_call_depth=test_case.max_call_depth)
            )
        )
        thread.start()
        thread.join()
    finally:
        threading.stack_size(previous)
    (result,) = results
    assert tuple(found.code for found in result.diagnostics) == test_case.expected_codes
    assert tuple(found.help for found in result.diagnostics) == test_case.expected_helps
    assert tuple(found.label for found in result.diagnostics) == test_case.expected_labels


@pytest.mark.parametrize(
    "test_case",
    (
        CallDepthRefusalTestCase(
            description="zero is refused",
            max_call_depth=0,
            expected_message="max_call_depth: .*SQLQuack's fixed nesting ceiling, not 0",
        ),
        CallDepthRefusalTestCase(
            description="one past the ceiling is refused",
            max_call_depth=513,
            expected_message="max_call_depth: .*SQLQuack's fixed nesting ceiling, not 513",
        ),
        CallDepthRefusalTestCase(
            description="1000 is refused, naming the ceiling",
            max_call_depth=1000,
            expected_message="max_call_depth: .*SQLQuack's fixed nesting ceiling, not 1000",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_call_depth_past_the_ceiling_when_checking_or_converting_then_it_is_refused(
    test_case: CallDepthRefusalTestCase,
) -> None:
    with pytest.raises(sqlquack.SqlQuackError, match=test_case.expected_message):
        sqlquack.check(sql="SELECT 1", max_call_depth=test_case.max_call_depth)
    with pytest.raises(sqlquack.SqlQuackError, match=test_case.expected_message):
        sqlquack.convert(
            sql="SELECT 1", options=sqlquack.ConvertOptions(max_call_depth=test_case.max_call_depth)
        )
