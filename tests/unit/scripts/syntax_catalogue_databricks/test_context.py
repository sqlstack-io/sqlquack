"""The audit compares a form inside its case context, without the context's wrapper parentheses."""

from __future__ import annotations

import pytest

from scripts.syntax_catalogue_databricks._helpers.audit import statement_context
from tests.unit.scripts.syntax_catalogue_databricks._test_types import ContextTestCase


@pytest.mark.parametrize(
    "test_case",
    (
        ContextTestCase(
            description=(
                "function: its cases run as SELECT ( <call> ), but a closing parenthesis after "
                "the form would cover FIRST ( … IGNORE NULLS ), which the functions/first page "
                "does not document"
            ),
            form={"id": "db.functions.first", "family": "functions"},
            expected_context=("SELECT ", ""),
        ),
        ContextTestCase(
            description="expression",
            form={"id": "db.expressions.case", "family": "expressions"},
            expected_context=("SELECT ", ""),
        ),
        ContextTestCase(
            description="table-valued function",
            form={"id": "db.functions.range", "family": "functions"},
            expected_context=("SELECT * FROM ", ""),
        ),
        ContextTestCase(
            description="form context",
            form={"id": "db.query.star", "family": "query"},
            expected_context=("SELECT ", " FROM t1"),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_form_when_its_context_is_taken_then_no_wrapper_parenthesis_closes_it(
    test_case: ContextTestCase,
) -> None:
    assert statement_context(test_case.form) == test_case.expected_context
