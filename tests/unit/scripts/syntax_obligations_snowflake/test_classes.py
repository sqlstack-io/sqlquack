"""Numeric operands of every Snowflake spelling get the integer class and its wrong-kind negative."""

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
grammar = ["SHOW TRANSACTIONS [ LIMIT <rows> ] [ TIMEOUT = <timeout_seconds> ] [ SIZE = <max_integer> ] [ NAME = <rows_name> ]"]
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
    ),
    ids=lambda case: case.description,
)
def test_given_a_numeric_operand_when_building_then_it_has_an_integer_and_a_wrong_kind_case(
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
