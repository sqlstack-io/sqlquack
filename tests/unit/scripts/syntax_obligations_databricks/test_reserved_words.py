"""The reserved-word cases follow the reserved-words page: only a table alias refuses ANTI."""

from __future__ import annotations

import pytest

from tests.unit.scripts.syntax_obligations_databricks._test_types import ReservedWordTestCase
from tests.unit.scripts.syntax_obligations_databricks.helpers import (
    class_cases,
    keyword_expects,
    refused_cases,
)


@pytest.mark.parametrize(
    "test_case",
    (
        ReservedWordTestCase(
            description="forms that apply to Databricks SQL",
            runtime_only=False,
            expected_keyword_expect="accept",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_the_built_cases_when_read_then_only_unquoted_anti_as_table_alias_is_refused(
    test_case: ReservedWordTestCase,
) -> None:
    found: list[tuple[str, str, str, str]] = class_cases(runtime_only=test_case.runtime_only)
    refused: list[tuple[str, str, str, str]] = refused_cases(found)

    assert any(label == "keyword" for label, _, _, _ in found)
    assert refused
    assert all(
        label == "reserved" and target.endswith("_alias>") and "`ANTI`" not in sql
        for label, target, _, sql in refused
    )
    assert all(expect == test_case.expected_keyword_expect for expect in keyword_expects(found))


@pytest.mark.parametrize(
    "test_case",
    (
        ReservedWordTestCase(
            description="forms documented for Databricks Runtime only, observed on the warehouse",
            runtime_only=True,
            expected_keyword_expect="observe",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_runtime_only_forms_when_read_then_their_keyword_cases_are_observed(
    test_case: ReservedWordTestCase,
) -> None:
    found: list[tuple[str, str, str, str]] = class_cases(runtime_only=test_case.runtime_only)

    assert found
    assert all(expect == test_case.expected_keyword_expect for expect in keyword_expects(found))
