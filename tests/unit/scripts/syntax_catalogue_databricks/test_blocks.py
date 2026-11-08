"""The audit's notation-block guard: a block with syntax notation needs a row; no row goes stale."""

from __future__ import annotations

import pytest

from scripts.syntax_catalogue_databricks._helpers.review import problems_of
from tests.unit.scripts.syntax_catalogue_databricks.helpers import (
    result_with_tokens,
    review_without,
)

BLOCK: str = "sql-ref-partition § PARTITION § 0150cb940614"
CHANGED: str = "sql-ref-partition § PARTITION § 9f0e6c1d2b3a"


@pytest.mark.parametrize(
    ("found", "rows", "expected"),
    (
        ((BLOCK,), (BLOCK,), []),
        ((BLOCK,), (), [f"unreviewed notation block: {BLOCK}"]),
        (
            (CHANGED,),
            (BLOCK,),
            [
                f"unreviewed notation block: {CHANGED}",
                f"notation block review names nothing current: {BLOCK}",
            ],
        ),
    ),
    ids=("reviewed", "a block without a row", "a block whose text changed"),
)
def test_given_notation_blocks_when_reviewed_then_a_missing_or_stale_row_is_a_problem(
    found: tuple[str, ...], rows: tuple[str, ...], expected: list[str]
) -> None:
    review = review_without(frozenset())
    review["block"] = [{"block": row, "reason": "catalogued"} for row in rows]

    problems: list[str] = problems_of(
        result=result_with_tokens(unused=frozenset(), extra=frozenset(), blocks=found),
        review=review,
        hashes={},
    )

    assert problems == expected
