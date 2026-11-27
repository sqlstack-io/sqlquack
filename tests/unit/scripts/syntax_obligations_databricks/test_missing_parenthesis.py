"""A case without a required `(` drops its `)` too, so only the missing `(` can refuse it."""

from __future__ import annotations

import pytest

from scripts.grammar_obligations.types import Row
from tests.unit.scripts.syntax_obligations_databricks._test_types import (
    BareFormTestCase,
    MissingOpenTestCase,
)
from tests.unit.scripts.syntax_obligations_databricks.helpers import (
    accepted_sql,
    missing_open_cases,
    missing_open_sql,
    unbalanced,
)


@pytest.mark.parametrize(
    "test_case",
    (
        MissingOpenTestCase(
            description="every missing-( case is balanced, quoted text aside",
            expected_unbalanced=(),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_the_built_cases_when_a_required_open_parenthesis_is_dropped_then_none_is_left_open(
    test_case: MissingOpenTestCase,
) -> None:
    cases: list[Row] = missing_open_cases()

    assert cases
    assert tuple(unbalanced(cases)) == test_case.expected_unbalanced


@pytest.mark.parametrize(
    "test_case",
    (
        BareFormTestCase(
            description="a name operand is replaced, so the call's name cannot read as a column with an alias",
            form="db.functions.position.position-in",
            expected_sql="SELECT ( POSITION 1 IN v1 )",
        ),
        BareFormTestCase(
            description="a keyword outside a choice stays as written",
            form="db.functions.trim.trim-both-from",
            expected_sql="SELECT ( TRIM BOTH FROM v1 )",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_call_without_its_open_parenthesis_when_built_then_its_first_operand_cannot_be_a_name(
    test_case: BareFormTestCase,
) -> None:
    assert test_case.expected_sql in missing_open_sql(test_case.form)


@pytest.mark.parametrize(
    "test_case",
    (
        BareFormTestCase(
            description="CURRENT_DATE without parentheses",
            form="db.functions.current-date",
            expected_sql="SELECT ( CURRENT_DATE )",
        ),
        BareFormTestCase(
            description="LOCALTIME without parentheses",
            form="db.functions.localtime",
            expected_sql="SELECT ( LOCALTIME )",
        ),
        BareFormTestCase(
            description="USER without parentheses",
            form="db.functions.user",
            expected_sql="SELECT ( USER )",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_function_documented_without_parentheses_when_built_then_its_bare_form_is_a_positive(
    test_case: BareFormTestCase,
) -> None:
    assert test_case.expected_sql in accepted_sql(test_case.form)
