"""The referee pending script, the project import and the comparison categories."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.referee_tools._helpers.classify import category
from scripts.referee_tools.constants import (
    BOTH_ACCEPT,
    BOTH_SYNTAX,
    CAUGHT_AT_PARSE,
    MISSED,
    NOT_RECORDED,
    TOO_LENIENT,
    TOO_STRICT,
)
from scripts.referee_tools.models import ParseVerdict
from tests.unit.scripts.referee_tools._test_types import (
    CatalogueTestCase,
    CategoryTestCase,
    PendingTestCase,
    ProjectTestCase,
)
from tests.unit.scripts.referee_tools.helpers import arities_of, import_project, pending_counts

_ACCEPTED: dict[str, object] = {"outcome": "accepted", "error": None}
_SYNTAX: dict[str, object] = {
    "outcome": "rejected",
    "error": {"errno": 1003, "sqlstate": "42000", "message": "syntax error line 1"},
}
_BINDING: dict[str, object] = {
    "outcome": "rejected",
    "error": {"errno": 904, "sqlstate": "42000", "message": "invalid identifier 'AMONT'"},
}


@pytest.mark.parametrize(
    "test_case",
    (
        PendingTestCase(
            description="a later selection replaces the earlier one's pending cases",
            selections=("generated/names.json", "generated/clauses.json"),
            expected_cases_from="generated/clauses.json",
            expected_environments=1,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_referee_files_when_writing_pending_then_each_selected_case_is_written_once(
    test_case: PendingTestCase, tmp_path: Path
) -> None:
    cases, environments, statements = pending_counts(
        repository=tmp_path, selections=test_case.selections, source=test_case.expected_cases_from
    )

    assert cases == statements, test_case.description
    assert environments == test_case.expected_environments, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        ProjectTestCase(
            description="a model reading a declared source becomes one statement",
            model_sql='-- totals; per order\nSELECT order_id, amount FROM __source("shop__orders")',
            expected_statements=1,
            expected_skipped={},
        ),
        ProjectTestCase(
            description="a model calling a project function is skipped",
            model_sql='SELECT __udf("order_score")(amount) AS s FROM __source("shop__orders")',
            expected_statements=0,
            expected_skipped={"calls a project function": 1},
        ),
        ProjectTestCase(
            description="a model reading an undeclared relation is skipped",
            model_sql='SELECT 1 AS n FROM __ref("missing_model")',
            expected_statements=0,
            expected_skipped={"reads a relation without declared column types": 1},
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_compiled_project_when_importing_then_usable_models_become_statements(
    test_case: ProjectTestCase, tmp_path: Path
) -> None:
    statements, summary = import_project(repository=tmp_path, model_sql=test_case.model_sql)

    assert len(statements) == test_case.expected_statements, test_case.description
    for reason, count in test_case.expected_skipped.items():
        assert f"{count} {reason}" in summary, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        CategoryTestCase("both accept", _ACCEPTED, True, BOTH_ACCEPT),
        CategoryTestCase("a parse error Snowflake accepts", _ACCEPTED, False, TOO_STRICT),
        CategoryTestCase("a syntax error SQLQuack parses", _SYNTAX, True, TOO_LENIENT),
        CategoryTestCase("a syntax error both reject", _SYNTAX, False, BOTH_SYNTAX),
        CategoryTestCase("a binding error rejected at parse", _BINDING, False, CAUGHT_AT_PARSE),
        CategoryTestCase("a binding error SQLQuack parses", _BINDING, True, MISSED),
        CategoryTestCase("no recorded outcome", None, True, NOT_RECORDED),
    ),
    ids=lambda case: case.description,
)
def test_given_an_outcome_and_a_parse_when_classifying_then_the_category_follows(
    test_case: CategoryTestCase,
) -> None:
    verdict: ParseVerdict = ParseVerdict(parses=test_case.parses, reason="")

    assert category(outcome=test_case.outcome, verdict=verdict) == test_case.expected_category


@pytest.mark.parametrize(
    "test_case",
    (
        CatalogueTestCase(
            description="overloads widen the bounds and -1 means no maximum",
            functions=[
                {
                    "name": "ROUND",
                    "arguments": "ROUND(NUMBER)",
                    "min_num_arguments": "1",
                    "max_num_arguments": "1",
                },
                {
                    "name": "ROUND",
                    "arguments": "ROUND(NUMBER, NUMBER)",
                    "min_num_arguments": "2",
                    "max_num_arguments": "2",
                },
                {
                    "name": "CONCAT",
                    "arguments": "CONCAT(VARCHAR)",
                    "min_num_arguments": "1",
                    "max_num_arguments": "-1",
                },
            ],
            expected_arities={"round": (1, 2), "concat": (1, None)},
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_recorded_catalogue_when_reading_arities_then_overloads_combine(
    test_case: CatalogueTestCase, tmp_path: Path
) -> None:
    arities: dict[str, object] = arities_of(directory=tmp_path, functions=test_case.functions)

    assert arities == test_case.expected_arities, test_case.description
