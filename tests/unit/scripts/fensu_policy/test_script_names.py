"""The script naming rules through Fensu's evaluation pipeline."""

from __future__ import annotations

import pytest
from fensu import RuleCase, RuleFile, RuleResult, evaluate_rule

from scripts.fensu_policy.rules.script_names import catalogue_spelling, no_unnamed_default_dialect
from tests.unit.scripts.fensu_policy._test_types import ScriptRuleTestCase

MAIN: str = "def main() -> int:\n    return 0\n"


@pytest.mark.parametrize(
    "test_case",
    [
        ScriptRuleTestCase(
            description="an unsuffixed script beside a dialect one faults",
            path="scripts/build_syntax_catalogue.py",
            source=MAIN,
            expected_fault_count=1,
            files=(RuleFile(path="scripts/build_syntax_catalogue_duckdb.py", source=MAIN),),
        ),
        ScriptRuleTestCase(
            description="naming every dialect passes",
            path="scripts/build_syntax_catalogue_snowflake.py",
            source=MAIN,
            expected_fault_count=0,
            files=(RuleFile(path="scripts/build_syntax_catalogue_duckdb.py", source=MAIN),),
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_scripts_when_checking_defaults_then_matches_contract(
    test_case: ScriptRuleTestCase,
) -> None:
    result: RuleResult = evaluate_rule(
        rule=no_unnamed_default_dialect,
        test_case=RuleCase(
            description=test_case.description,
            path=test_case.path,
            source=test_case.source,
            expected_fault_count=test_case.expected_fault_count,
            files=test_case.files,
            scope=test_case.scope,
            scope_root=test_case.scope_root,
        ),
    )

    assert result.fault_count == test_case.expected_fault_count


@pytest.mark.parametrize(
    "test_case",
    [
        ScriptRuleTestCase(
            description="a catalog script faults",
            path="scripts/extract_duckdb_catalog.py",
            source=MAIN,
            expected_fault_count=1,
        ),
        ScriptRuleTestCase(
            description="a catalogue script passes",
            path="scripts/extract_duckdb_catalogue.py",
            source=MAIN,
            expected_fault_count=0,
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_scripts_when_checking_spelling_then_matches_contract(
    test_case: ScriptRuleTestCase,
) -> None:
    result: RuleResult = evaluate_rule(
        rule=catalogue_spelling,
        test_case=RuleCase(
            description=test_case.description,
            path=test_case.path,
            source=test_case.source,
            expected_fault_count=test_case.expected_fault_count,
            files=test_case.files,
            scope=test_case.scope,
            scope_root=test_case.scope_root,
        ),
    )

    assert result.fault_count == test_case.expected_fault_count
