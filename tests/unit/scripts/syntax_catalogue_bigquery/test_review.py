"""The audit review fails closed: a deleted row, a stale rule or a contradicted kind fails."""

from __future__ import annotations

from dataclasses import replace

import pytest

from scripts.syntax_catalogue_bigquery._helpers.review import problems_of
from scripts.syntax_catalogue_bigquery.models import AuditResult
from tests.unit.scripts.syntax_catalogue_bigquery._test_types import DocsTestCase, ReviewTestCase
from tests.unit.scripts.syntax_catalogue_bigquery.helpers import (
    result_matching_review,
    review_without,
    sequences_of,
)


@pytest.mark.parametrize(
    "test_case",
    (
        ReviewTestCase(
            description="the committed rows",
            removed=(),
            unwalked_rules=frozenset(),
            expected_problems=[],
        ),
        ReviewTestCase(
            description="the EXPLAIN gap row deleted",
            removed=(("gap", "rule", "explain_statement"),),
            unwalked_rules=frozenset(),
            expected_problems=["unreviewed gap: explain_statement: EXPLAIN"],
        ),
        ReviewTestCase(
            description="the Spanner INTERLEAVE IN PARENT gap row deleted",
            removed=(("gap", "rule", "opt_spanner_interleave_in_parent_clause"),),
            unwalked_rules=frozenset(),
            expected_problems=[
                "unreviewed gap: opt_spanner_interleave_in_parent_clause: INTERLEAVE IN PARENT"
            ],
        ),
        ReviewTestCase(
            description="the FEATURE_TTL row deleted",
            removed=(("feature", "name", "FEATURE_TTL"),),
            unwalked_rules=frozenset(),
            expected_problems=["unreviewed language feature: FEATURE_TTL"],
        ),
        ReviewTestCase(
            description="the EPOCH dispatched-word row deleted",
            removed=(("dispatched", "word", "EPOCH"),),
            unwalked_rules=frozenset(),
            expected_problems=["unreviewed dispatched word: EPOCH"],
        ),
        ReviewTestCase(
            description="the multi-sequence DROP gap row deleted",
            removed=(("gap", "rule", "drop_statement"),),
            unwalked_rules=frozenset(),
            expected_problems=sorted(
                f"unreviewed gap: drop_statement: {s}" for s in sequences_of("drop_statement")
            ),
        ),
        ReviewTestCase(
            description="the hint [[unseen]] row deleted",
            removed=(("unseen", "rule", "hint"),),
            unwalked_rules=frozenset(),
            expected_problems=["unreviewed unseen rule: hint"],
        ),
        ReviewTestCase(
            description="an [[unseen]] row whose rule the walk no longer reaches",
            removed=(),
            unwalked_rules=frozenset({"cast_expression"}),
            expected_problems=["unseen rule the walk does not reach: cast_expression"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_reviewed_rows_when_one_is_deleted_then_the_audit_reports_it(
    test_case: ReviewTestCase,
) -> None:
    problems: list[str] = problems_of(
        result=result_matching_review(test_case.unwalked_rules),
        review=review_without(test_case.removed),
        hashes={},
    )

    assert problems == test_case.expected_problems


@pytest.mark.parametrize(
    "test_case",
    (
        DocsTestCase(
            description="no page spells a googlesql_only row",
            extra_streams=[],
            expected_problems=[],
        ),
        DocsTestCase(
            description="a page spells EXPLAIN, reviewed googlesql_only as a gap and a keyword",
            extra_streams=[("doc.x", ["EXPLAIN"])],
            expected_problems=[
                "gap explain_statement: reviewed googlesql_only, but doc.x spells EXPLAIN",
                "keyword row ['ABORT', 'ACTION', 'ALIGN']: reviewed googlesql_only, "
                "but doc.x spells EXPLAIN",
            ],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_googlesql_only_row_when_the_docs_spell_it_then_its_kind_is_a_problem(
    test_case: DocsTestCase,
) -> None:
    result: AuditResult = result_matching_review(frozenset())
    spelled: AuditResult = replace(
        result, doc_streams=[*result.doc_streams, *test_case.extra_streams]
    )

    problems: list[str] = problems_of(result=spelled, review=review_without(()), hashes={})

    assert problems == test_case.expected_problems
