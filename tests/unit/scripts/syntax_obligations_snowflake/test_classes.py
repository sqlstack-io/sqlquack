"""Snowflake operands named as numbers, objects or words get their class and its wrong-kind negative."""

from __future__ import annotations

import tomllib
from pathlib import Path

import pytest

from scripts.grammar_obligations.constants import KIND_NEGATIVE, KIND_OPERAND_CLASS
from scripts.syntax_obligations_snowflake.main.build import run_build
from tests.unit.scripts.syntax_obligations_snowflake._test_types import OperandClassTestCase

FAMILY: str = "transaction_session"
CATALOGUE: str = """
[[form]]
id = "sf.transaction_session.show-transactions"
family = "transaction_session"
kind = "statement"
grammar = ["SHOW TRANSACTIONS [ LIMIT <rows> ] [ TIMEOUT = <timeout_seconds> ] [ SIZE = <max_integer> ] [ NAME = <rows_name> ] [ PURPOSE = <purpose> ] [ OWNER = <role> ] [ AGE = <days> ] [ SCALE = <fractional_seconds_precision> ] [ TAG = <purposes> ]"]
uses = []
status = "catalogued"
"""


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
    ),
    ids=lambda case: case.description,
)
def test_given_a_named_operand_when_building_then_it_has_its_class_and_a_wrong_kind_case(
    tmp_path: Path, test_case: OperandClassTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / f"{FAMILY}.toml").write_text(CATALOGUE)
    (tmp_path / "limits.toml").write_text("")

    assert run_build(["--root", str(tmp_path), FAMILY]) == 0

    rows: list[dict[str, str]] = tomllib.loads(
        (tmp_path / "obligations" / f"{FAMILY}.toml").read_text()
    )["obligation"]
    notes: dict[tuple[str, str], str] = {
        (row["kind"], row.get("target", "")): row.get("note", "") for row in rows
    }
    assert {
        notes.get((KIND_OPERAND_CLASS, test_case.target), ""),
        notes.get((KIND_NEGATIVE, test_case.target), ""),
    } == test_case.expected_notes
