"""The audit's notation-block guard: a block with syntax notation needs a row; no row goes stale."""

from __future__ import annotations

import pytest

from scripts.syntax_catalogue_databricks._helpers.review import problems_of
from scripts.syntax_catalogue_databricks.types import Review
from tests.unit.scripts.syntax_catalogue_databricks._test_types import BlockTestCase
from tests.unit.scripts.syntax_catalogue_databricks.helpers import (
    result_with_tokens,
    review_without,
)

BLOCK: str = "sql-ref-partition § PARTITION § 0150cb940614"
CHANGED: str = "sql-ref-partition § PARTITION § 9f0e6c1d2b3a"


@pytest.mark.parametrize(
    "test_case",
    (
        BlockTestCase(description="reviewed", found=(BLOCK,), rows=(BLOCK,), expected_problems=[]),
        BlockTestCase(
            description="a block without a row",
            found=(BLOCK,),
            rows=(),
            expected_problems=[f"unreviewed notation block: {BLOCK}"],
        ),
        BlockTestCase(
            description="a block whose text changed",
            found=(CHANGED,),
            rows=(BLOCK,),
            expected_problems=[
                f"unreviewed notation block: {CHANGED}",
                f"notation block review names nothing current: {BLOCK}",
            ],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_notation_blocks_when_reviewed_then_a_missing_or_stale_row_is_a_problem(
    test_case: BlockTestCase,
) -> None:
    review: Review = review_without(frozenset())
    review["block"] = [{"block": row, "reason": "catalogued"} for row in test_case.rows]

    problems: list[str] = problems_of(
        result=result_with_tokens(unused=frozenset(), extra=frozenset(), blocks=test_case.found),
        review=review,
        hashes={},
    )

    assert problems == test_case.expected_problems
