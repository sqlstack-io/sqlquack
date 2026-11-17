"""A case's setup adds the fixture tables its text names, after the shared setup, and no others."""

from __future__ import annotations

import pytest

from scripts.syntax_obligations_databricks._helpers.cases import case_setup
from scripts.syntax_obligations_databricks.constants import CASE_FIXTURES, SETUP
from tests.unit.scripts.syntax_obligations_databricks._test_types import CaseSetupTestCase


@pytest.mark.parametrize(
    "test_case",
    (
        CaseSetupTestCase(
            description="both named, in declared order",
            sql=(
                "INSERT INTO TABLE students AS t REPLACE ON t.name <=> s.name "
                "( SELECT * FROM people ) AS s"
            ),
            expected_fixtures=(CASE_FIXTURES["students"], CASE_FIXTURES["people"]),
        ),
        CaseSetupTestCase(
            description="a name in any case",
            sql="SELECT * FROM PEOPLE",
            expected_fixtures=(CASE_FIXTURES["people"],),
        ),
        CaseSetupTestCase(
            description="a longer name is no fixture",
            sql="SELECT * FROM people_t1",
            expected_fixtures=(),
        ),
        CaseSetupTestCase(
            description="no fixture named",
            sql="SELECT * FROM t1",
            expected_fixtures=(),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_case_text_when_its_setup_is_built_then_it_adds_only_the_fixtures_named(
    test_case: CaseSetupTestCase,
) -> None:
    assert case_setup(test_case.sql) == [*SETUP, *test_case.expected_fixtures]
