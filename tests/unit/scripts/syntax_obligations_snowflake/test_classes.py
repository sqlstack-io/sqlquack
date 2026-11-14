"""Snowflake operands named as numbers, objects or words, quoted values built from placeholders, and
class instance names, LIKE patterns and tags get their class and its wrong-kind negative."""

from __future__ import annotations

import tomllib
from pathlib import Path

import pytest

from scripts.grammar_obligations.constants import KIND_NEGATIVE, KIND_OPERAND_CLASS
from scripts.syntax_obligations_snowflake.main.build import run_build
from tests.unit.scripts.syntax_obligations_snowflake._test_types import (
    DerivedLimitTestCase,
    OperandClassTestCase,
)

CATALOGUES: dict[str, str] = {
    "transaction_session": """
[[form]]
id = "sf.transaction_session.show-transactions"
family = "transaction_session"
kind = "statement"
grammar = ["SHOW TRANSACTIONS [ LIMIT <rows> ] [ TIMEOUT = <timeout_seconds> ] [ SIZE = <max_integer> ] [ NAME = <rows_name> ] [ PURPOSE = <purpose> ] [ OWNER = <role> ] [ AGE = <days> ] [ SCALE = <fractional_seconds_precision> ] [ TAG = <purposes> ] [ PERIOD = '<num> { seconds | minutes }' ]"]
uses = []
status = "catalogued"
""",
    "classes": """
[[form]]
id = "sf.classes.budget.commands.show-budget"
family = "classes"
kind = "statement"
grammar = ["SHOW SNOWFLAKE.CORE.BUDGET [ LIKE '<pattern>' ]"]
uses = []
status = "catalogued"

[[form]]
id = "sf.classes.budget.methods.set-notification-threshold"
family = "classes"
kind = "statement"
grammar = ["<budget_name>!SET_NOTIFICATION_THRESHOLD( <number> )"]
uses = []
status = "catalogued"

[[form]]
id = "sf.classes.forecast.commands.create-forecast"
family = "classes"
kind = "statement"
grammar = ["CREATE SNOWFLAKE.ML.FORECAST <model_name> ( INPUT_DATA => <input_data> ) [ TAG ( <tag_name> = '<tag_value>' ) ]"]
uses = []
status = "catalogued"
""",
}


@pytest.mark.parametrize(
    "test_case",
    (
        OperandClassTestCase(
            description="a row count is an integer",
            target="<rows> in [ LIMIT <rows> ]",
            expected_notes=frozenset(
                {"a valid integer_literal", "a string literal in place of <rows>"}
            ),
        ),
        OperandClassTestCase(
            description="a number of seconds is an integer",
            target="<timeout_seconds> in [ TIMEOUT = <timeout_seconds> ]",
            expected_notes=frozenset(
                {"a valid integer_literal", "a string literal in place of <timeout_seconds>"}
            ),
        ),
        OperandClassTestCase(
            description="an operand named as an integer is one",
            target="<max_integer> in [ SIZE = <max_integer> ]",
            expected_notes=frozenset(
                {"a valid integer_literal", "a string literal in place of <max_integer>"}
            ),
        ),
        OperandClassTestCase(
            description="a name that starts with rows stays a name",
            target="<rows_name> in [ NAME = <rows_name> ]",
            expected_notes=frozenset(
                {"a valid identifier", "a string literal in place of <rows_name>"}
            ),
        ),
        OperandClassTestCase(
            description="a contact purpose is a bare word",
            target="<purpose> in [ PURPOSE = <purpose> ]",
            expected_notes=frozenset({"a valid word", "a string literal in place of <purpose>"}),
        ),
        OperandClassTestCase(
            description="a role the page names without a _name suffix is an identifier",
            target="<role> in [ OWNER = <role> ]",
            expected_notes=frozenset({"a valid identifier", "a string literal in place of <role>"}),
        ),
        OperandClassTestCase(
            description="a number of days is an integer",
            target="<days> in [ AGE = <days> ]",
            expected_notes=frozenset(
                {"a valid integer_literal", "a string literal in place of <days>"}
            ),
        ),
        OperandClassTestCase(
            description="a prefixed precision is an integer",
            target="<fractional_seconds_precision> in [ SCALE = <fractional_seconds_precision> ]",
            expected_notes=frozenset(
                {
                    "a valid integer_literal",
                    "a string literal in place of <fractional_seconds_precision>",
                }
            ),
        ),
        OperandClassTestCase(
            description="a name that only starts with a word name stays unclassified",
            target="<purposes> in [ TAG = <purposes> ]",
            expected_notes=frozenset({"a valid operand", ""}),
        ),
        OperandClassTestCase(
            description="a quoted value built from placeholders is a string",
            target="'<num> { seconds | minutes }' in [ PERIOD = '<num> { seconds | minutes }' ]",
            expected_notes=frozenset(
                {
                    "a valid string_literal",
                    "an unquoted identifier in place of '<num> { seconds | minutes }'",
                }
            ),
        ),
        OperandClassTestCase(
            description="a class's LIKE pattern is a string, not a call argument",
            target="'<pattern>' in [ LIKE '<pattern>' ]",
            expected_notes=frozenset(
                {"a valid string_literal", "an unquoted identifier in place of '<pattern>'"}
            ),
            family="classes",
        ),
        OperandClassTestCase(
            description="a method's instance is a name, not a call argument",
            target="<budget_name>",
            expected_notes=frozenset(
                {"a valid identifier", "a string literal in place of <budget_name>"}
            ),
            family="classes",
        ),
        OperandClassTestCase(
            description="a class's tag list is a clause, not a call",
            target="<tag_name> in [ TAG ( <tag_name> = '<tag_value>' ) ]",
            expected_notes=frozenset(
                {"a valid identifier", "a string literal in place of <tag_name>"}
            ),
            family="classes",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_named_operand_when_building_then_it_has_its_class_and_a_wrong_kind_case(
    tmp_path: Path, test_case: OperandClassTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / f"{test_case.family}.toml").write_text(CATALOGUES[test_case.family])
    (tmp_path / "limits.toml").write_text("")

    assert run_build(["--root", str(tmp_path), test_case.family]) == 0

    rows: list[dict[str, str]] = tomllib.loads(
        (tmp_path / "obligations" / f"{test_case.family}.toml").read_text()
    )["obligation"]
    notes: dict[tuple[str, str], str] = {
        (row["kind"], row.get("target", "")): row.get("note", "") for row in rows
    }
    assert {
        notes.get((KIND_OPERAND_CLASS, test_case.target), ""),
        notes.get((KIND_NEGATIVE, test_case.target), ""),
    } == test_case.expected_notes


@pytest.mark.parametrize(
    "test_case",
    (
        DerivedLimitTestCase(
            description="a quoted placeholder value's negative",
            target="'<num> { seconds | minutes }' in [ PERIOD = '<num> { seconds | minutes }' ]",
            sql="SHOW TRANSACTIONS PERIOD = m1 ;",
            expected_count=1,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_limit_that_writes_a_derived_obligation_when_building_then_it_is_listed_once(
    tmp_path: Path, test_case: DerivedLimitTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / "transaction_session.toml").write_text(
        CATALOGUES["transaction_session"]
    )
    note: str = test_case.target.split(" in ", 1)[0]
    (tmp_path / "limits.toml").write_text(
        f"""
[[limit]]
form = "sf.transaction_session.show-transactions"
kind = "{KIND_NEGATIVE}"
target = "{test_case.target}"
note = "an unquoted identifier in place of {note}"
sql = "{test_case.sql}"
expect = "reject"
context = "raw"
"""
    )

    assert run_build(["--root", str(tmp_path), "transaction_session"]) == 0

    rows: list[dict[str, str]] = tomllib.loads(
        (tmp_path / "obligations" / "transaction_session.toml").read_text()
    )["obligation"]
    pairs: list[tuple[str, str]] = [(row["kind"], row.get("target", "")) for row in rows]
    assert pairs.count((KIND_NEGATIVE, test_case.target)) == test_case.expected_count
