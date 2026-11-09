"""Operand classes: graph and row patterns are names or grammars, a LIKE pattern is a string."""

from __future__ import annotations

import pytest

from scripts.grammar_obligations._helpers import obligations
from scripts.grammar_obligations.constants import KIND_OPERAND_CLASS
from scripts.grammar_obligations.models import Draft
from scripts.syntax_obligations_bigquery._helpers.adapt import install
from tests.unit.scripts.syntax_obligations_bigquery._test_types import ClassTestCase
from tests.unit.scripts.syntax_obligations_bigquery.helpers import ROOT

NO_CLASS: str = ""


@pytest.mark.parametrize(
    "test_case",
    (
        ClassTestCase(
            description="a graph pattern variable is a name",
            form="bq.expressions.operators.all-different-predicate",
            grammar="ALL_DIFFERENT ( <graph_pattern_variable> , <graph_pattern_variable> )",
            placeholder="<graph_pattern_variable>",
            expected_class="a valid identifier",
        ),
        ClassTestCase(
            description="a node pattern is its own grammar",
            form="bq.query.gql-element-pattern.element-pattern",
            grammar="<element_pattern> ::= { <node_pattern> | <edge_pattern> }",
            placeholder="<node_pattern>",
            expected_class=NO_CLASS,
        ),
        ClassTestCase(
            description="MATCH_RECOGNIZE's row pattern is its own grammar",
            form="bq.query.match-recognize",
            grammar="FROM <from_item> MATCH_RECOGNIZE ( PATTERN ( <pattern> ) )",
            placeholder="<pattern>",
            expected_class=NO_CLASS,
        ),
        ClassTestCase(
            description="the pipe MATCH_RECOGNIZE's row pattern is its own grammar",
            form="bq.query.match-recognize-pipe",
            grammar="|> MATCH_RECOGNIZE ( PATTERN ( <pattern> ) )",
            placeholder="<pattern>",
            expected_class=NO_CLASS,
        ),
        ClassTestCase(
            description="a LIKE pattern stays a string",
            form="bq.expressions.operators.like-operator",
            grammar="<expression> [ NOT ] LIKE <pattern>",
            placeholder="<pattern>",
            expected_class="a valid string_literal",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_placeholder_when_deriving_then_it_takes_its_documented_class(
    test_case: ClassTestCase,
) -> None:
    install(ROOT)
    drafts: list[Draft] = obligations.derive_trees(
        form=test_case.form, grammar=[test_case.grammar]
    )[0]
    notes: dict[tuple[str, str], str] = {(draft.kind, draft.target): draft.note for draft in drafts}
    assert notes.get((KIND_OPERAND_CLASS, test_case.placeholder), NO_CLASS) == (
        test_case.expected_class
    )
