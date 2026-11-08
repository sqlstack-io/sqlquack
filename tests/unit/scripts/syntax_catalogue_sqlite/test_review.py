"""The audit fails closed: a deleted review row, an unreached unseen rule or another parser."""

from __future__ import annotations

import pytest

from scripts.syntax_catalogue_sqlite._helpers.review import problems_of
from tests.unit.scripts.syntax_catalogue_sqlite._test_types import (
    RemovedRowTestCase,
    WalkTestCase,
)
from tests.unit.scripts.syntax_catalogue_sqlite.helpers import (
    committed_review,
    review_without,
    reviewed_result,
)


@pytest.mark.parametrize(
    "test_case",
    (
        RemovedRowTestCase(
            description="the ATTACH ... KEY unseen row deleted",
            table="unseen",
            key="rule",
            removed="key_opt",
            expected_problems=["unreviewed unseen rule: key_opt"],
        ),
        RemovedRowTestCase(
            description="the NULL column constraint unseen row deleted",
            table="unseen",
            key="rule",
            removed="ccons",
            expected_problems=["unreviewed unseen rule: ccons"],
        ),
        RemovedRowTestCase(
            description="the TRANSACTION <name> unseen row deleted",
            table="unseen",
            key="rule",
            removed="trans_opt",
            expected_problems=["unreviewed unseen rule: trans_opt"],
        ),
        RemovedRowTestCase(
            description="the ON INSERT gap row deleted",
            table="gap",
            key="rule",
            removed="refarg",
            expected_problems=[
                "unreviewed gap: refarg: ON INSERT CASCADE",
                "unreviewed gap: refarg: ON INSERT NO ACTION",
                "unreviewed gap: refarg: ON INSERT RESTRICT",
                "unreviewed gap: refarg: ON INSERT SET DEFAULT",
                "unreviewed gap: refarg: ON INSERT SET NULL",
            ],
        ),
        RemovedRowTestCase(
            description="the WITHIN keyword row deleted",
            table="keyword",
            key="keyword",
            removed="WITHIN",
            expected_problems=["unreviewed keyword: WITHIN"],
        ),
        RemovedRowTestCase(
            description="the subtype() function row deleted",
            table="function",
            key="name",
            removed="subtype",
            expected_problems=["unreviewed function: subtype"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_review_row_deleted_when_reviewed_then_its_finding_is_a_problem(
    test_case: RemovedRowTestCase,
) -> None:
    problems: list[str] = problems_of(
        result=reviewed_result(),
        review=review_without(table=test_case.table, key=test_case.key, removed=test_case.removed),
        hashes={},
    )

    assert problems == test_case.expected_problems


@pytest.mark.parametrize(
    "test_case",
    (
        WalkTestCase(
            description="the committed rows against the pinned walk",
            unwalked_rules=frozenset(),
            productions_read=412,
            expected_problems=[],
        ),
        WalkTestCase(
            description="an unseen rule the walk no longer reaches",
            unwalked_rules=frozenset({"key_opt"}),
            productions_read=412,
            expected_problems=["unseen rule the walk does not reach: key_opt"],
        ),
        WalkTestCase(
            description="parse.y read differently from the generated parser",
            unwalked_rules=frozenset(),
            productions_read=410,
            expected_problems=[
                "parse.y read as 410 productions; the amalgamation's parser has 412"
            ],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_the_walk_when_reviewed_then_an_unreached_row_or_another_parser_is_a_problem(
    test_case: WalkTestCase,
) -> None:
    problems: list[str] = problems_of(
        result=reviewed_result(
            unwalked=test_case.unwalked_rules, productions_read=test_case.productions_read
        ),
        review=committed_review(),
        hashes={},
    )

    assert problems == test_case.expected_problems
