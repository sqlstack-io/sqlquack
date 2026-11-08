"""The audit's [[unseen]] guard: every uncompared lexer token needs a row, and no row goes stale."""

from __future__ import annotations

import pytest

from scripts.syntax_catalogue_databricks._helpers.review import problems_of
from tests.unit.scripts.syntax_catalogue_databricks._test_types import UnseenTestCase
from tests.unit.scripts.syntax_catalogue_databricks.helpers import (
    result_with_tokens,
    review_without,
)


@pytest.mark.parametrize(
    "test_case",
    (
        UnseenTestCase(
            description="the committed rows",
            removed_tokens=frozenset(),
            unused_tokens=frozenset(),
            extra_tokens=frozenset(),
            expected_problems=[],
        ),
        UnseenTestCase(
            description="the back-quoted identifier row deleted",
            removed_tokens=frozenset({"BACKQUOTED_IDENTIFIER"}),
            unused_tokens=frozenset(),
            extra_tokens=frozenset(),
            expected_problems=["unreviewed unseen token: BACKQUOTED_IDENTIFIER"],
        ),
        UnseenTestCase(
            description="the hint opener row deleted",
            removed_tokens=frozenset({"HENT_START"}),
            unused_tokens=frozenset(),
            extra_tokens=frozenset(),
            expected_problems=["unreviewed unseen token: HENT_START"],
        ),
        UnseenTestCase(
            description="a listed token the walked grammar no longer uses",
            removed_tokens=frozenset(),
            unused_tokens=frozenset({"QUESTION"}),
            extra_tokens=frozenset(),
            expected_problems=["unseen token the walked grammar does not use: QUESTION"],
        ),
        UnseenTestCase(
            description="a new uncompared token the list does not name",
            removed_tokens=frozenset(),
            unused_tokens=frozenset(),
            extra_tokens=frozenset({"NEW_LITERAL"}),
            expected_problems=["uncompared token without an [[unseen]] entry: NEW_LITERAL"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_unseen_rows_when_reviewed_then_a_missing_or_stale_row_is_a_problem(
    test_case: UnseenTestCase,
) -> None:
    problems: list[str] = problems_of(
        result=result_with_tokens(unused=test_case.unused_tokens, extra=test_case.extra_tokens),
        review=review_without(test_case.removed_tokens),
        hashes={},
    )

    assert problems == test_case.expected_problems
