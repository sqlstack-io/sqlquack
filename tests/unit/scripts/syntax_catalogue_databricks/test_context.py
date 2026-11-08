"""The audit compares a form inside its case context, without the context's wrapper parentheses."""

from __future__ import annotations

import pytest

from scripts.syntax_catalogue_databricks._helpers.audit import statement_context


@pytest.mark.parametrize(
    ("form", "expected"),
    (
        # A function's cases run as SELECT ( <call> ); a closing parenthesis after the form would
        # cover FIRST ( … IGNORE NULLS ), which the functions/first page does not document.
        ({"id": "db.functions.first", "family": "functions"}, ("SELECT ", "")),
        ({"id": "db.expressions.case", "family": "expressions"}, ("SELECT ", "")),
        ({"id": "db.functions.range", "family": "functions"}, ("SELECT * FROM ", "")),
        ({"id": "db.query.star", "family": "query"}, ("SELECT ", " FROM t1")),
    ),
    ids=("function", "expression", "table-valued function", "form context"),
)
def test_given_a_form_when_its_context_is_taken_then_no_wrapper_parenthesis_closes_it(
    form: dict[str, object], expected: tuple[str, str]
) -> None:
    assert statement_context(form) == expected
