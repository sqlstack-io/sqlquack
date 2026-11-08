"""The audit's [[unseen]] guard: every required blind-spot row must be present and still walked."""

from __future__ import annotations

import pytest

from scripts.syntax_catalogue_duckdb._helpers.review import problems_of
from tests.unit.scripts.syntax_catalogue_duckdb._test_types import UnseenTestCase
from tests.unit.scripts.syntax_catalogue_duckdb.helpers import (
    result_walking_all_but,
    review_without,
)


@pytest.mark.parametrize(
    "test_case",
    (
        UnseenTestCase(
            description="the committed rows",
            removed_rows=frozenset(),
            unwalked_rules=frozenset(),
            expected_problems=[],
        ),
        UnseenTestCase(
            description="the CREATE TABLE ... WITH row deleted",
            removed_rows=frozenset({"CreateColumnList"}),
            unwalked_rules=frozenset(),
            expected_problems=["unreviewed unseen rule: CreateColumnList"],
        ),
        UnseenTestCase(
            description="the CREATE TEMPORARY TYPE row deleted",
            removed_rows=frozenset({"CreateStatement"}),
            unwalked_rules=frozenset(),
            expected_problems=["unreviewed unseen rule: CreateStatement"],
        ),
        UnseenTestCase(
            description="the CREATE VIEW ... WITH ( DEFER_BINDING ) row deleted",
            removed_rows=frozenset({"CreateViewStmt"}),
            unwalked_rules=frozenset(),
            expected_problems=["unreviewed unseen rule: CreateViewStmt"],
        ),
        UnseenTestCase(
            description="the PIVOT ... IN <enum-type> row deleted",
            removed_rows=frozenset({"PivotEnumTarget"}),
            unwalked_rules=frozenset(),
            expected_problems=["unreviewed unseen rule: PivotEnumTarget"],
        ),
        UnseenTestCase(
            description="a row whose rule the walk no longer reaches",
            removed_rows=frozenset(),
            unwalked_rules=frozenset({"CreateViewStmt"}),
            expected_problems=["unseen rule the walk does not reach: CreateViewStmt"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_unseen_rows_when_reviewed_then_a_missing_or_stale_row_is_a_problem(
    test_case: UnseenTestCase,
) -> None:
    problems: list[str] = problems_of(
        result=result_walking_all_but(test_case.unwalked_rules),
        review=review_without(test_case.removed_rows),
        hashes={},
    )

    assert problems == test_case.expected_problems
