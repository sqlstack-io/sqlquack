"""Snowflake operands named as numbers, objects or words, quoted values built from placeholders, and
class instance names, LIKE patterns and tags get their class and its wrong-kind negative."""

from __future__ import annotations

import tomllib
from pathlib import Path

import pytest

from scripts.grammar_obligations._helpers.obligations import family_obligations
from scripts.grammar_obligations._helpers.render import style_of, submissions
from scripts.grammar_obligations.constants import (
    KIND_NEGATIVE,
    KIND_OPERAND_CLASS,
    KIND_OPERAND_MISSING,
)
from scripts.grammar_obligations.models import Draft
from scripts.syntax_obligations_snowflake._helpers.rules import install
from scripts.syntax_obligations_snowflake.main.build import run_build
from tests.unit.scripts.syntax_obligations_snowflake._test_types import (
    DerivedLimitTestCase,
    DocumentedRefusalTestCase,
    ListItemTestCase,
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

[[form]]
id = "sf.transaction_session.drop-tags"
family = "transaction_session"
kind = "statement"
grammar = ["DROP TAG <name> [ , <name> ... ]"]
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

DROP_TAGS: str = "DROP TAG <name> [ , <name> ... ]"
PACKAGES_POLICY: str = (
    "CREATE PACKAGES POLICY <name> LANGUAGE PYTHON"
    " [ ALLOWLIST = ( [ '<packageSpec>' ] [ , '<packageSpec>' ... ] ) ]"
)
VALUES: str = "SELECT ... FROM ( VALUES ( <expr> [ , <expr> [ , ... ] ] ) [ , ( ... ) ] )"


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


@pytest.mark.parametrize(
    "test_case",
    (
        DocumentedRefusalTestCase(
            description="a comment Snowflake refuses that the option's positive also renders",
            grammar="CREATE TAG <name> [ COMMENT = '<string_literal>' ]",
            target="'<string_literal>' in [ COMMENT = '<string_literal>' ]",
            sql="CREATE TAG v1 COMMENT = 'm' ;",
            expected_note="a comment Snowflake refuses on a tag, observed",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_documented_refusal_a_positive_also_renders_when_building_then_it_stays_observed(
    tmp_path: Path, test_case: DocumentedRefusalTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / "transaction_session.toml").write_text(
        f"""
[[form]]
id = "sf.transaction_session.create-commented-tag"
family = "transaction_session"
kind = "statement"
grammar = ["{test_case.grammar}"]
uses = []
status = "catalogued"
"""
    )
    (tmp_path / "limits.toml").write_text(
        f"""
[[limit]]
form = "sf.transaction_session.create-commented-tag"
kind = "{KIND_NEGATIVE}"
target = "{test_case.target}"
note = "{test_case.expected_note}"
sql = "{test_case.sql}"
expect = "observe"
documented = true
context = "raw"
"""
    )

    assert run_build(["--root", str(tmp_path), "transaction_session"]) == 0

    rows: list[dict[str, str]] = tomllib.loads(
        (tmp_path / "obligations" / "transaction_session.toml").read_text()
    )["obligation"]
    assert [row.get("note") for row in rows].count(test_case.expected_note) == 1
    assert not (tmp_path / "withdrawn" / "transaction_session.toml").exists()


@pytest.mark.parametrize(
    "test_case",
    (
        ListItemTestCase(
            description="the second item is missing after its separator",
            family="transaction_session",
            grammar=DROP_TAGS,
            kind=KIND_OPERAND_MISSING,
            label="<name> in [ , <name> ... ]",
            expected_sql=("DROP TAG v1 , ;",),
        ),
        ListItemTestCase(
            description="the second item is a valid name of its own",
            family="transaction_session",
            grammar=DROP_TAGS,
            kind=KIND_OPERAND_CLASS,
            label="<name> in [ , <name> ... ]",
            expected_sql=("DROP TAG v1 , v1 ;",),
        ),
        ListItemTestCase(
            description="the item after an optional first item is missing after its separator",
            family="security_admin",
            grammar=PACKAGES_POLICY,
            kind=KIND_OPERAND_MISSING,
            label="'<packageSpec>' in [ , '<packageSpec>' ... ]",
            expected_sql=("CREATE PACKAGES POLICY v1 LANGUAGE PYTHON ALLOWLIST = ( 'm' , ) ;",),
        ),
        ListItemTestCase(
            description="the item after an optional first item is a valid string of its own",
            family="security_admin",
            grammar=PACKAGES_POLICY,
            kind=KIND_OPERAND_CLASS,
            label="'<packageSpec>' in [ , '<packageSpec>' ... ]",
            expected_sql=("CREATE PACKAGES POLICY v1 LANGUAGE PYTHON ALLOWLIST = ( 'm' , 'm' ) ;",),
        ),
        ListItemTestCase(
            description="an elided row loses its opening parenthesis",
            family="query",
            grammar=VALUES,
            kind=KIND_NEGATIVE,
            label="the form without its required ( in [ , ( ... ) ]",
            expected_sql=("SELECT 1 FROM ( VALUES ( 1 ) , 2 ) ) ;",),
        ),
        ListItemTestCase(
            description="an elided row loses its closing parenthesis",
            family="query",
            grammar=VALUES,
            kind=KIND_NEGATIVE,
            label="the form without its required ) in [ , ( ... ) ]",
            expected_sql=("SELECT 1 FROM ( VALUES ( 1 ) , ( 2 ) ;",),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_repeated_list_item_when_building_then_the_second_item_has_its_own_cases(
    tmp_path: Path, test_case: ListItemTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / f"{test_case.family}.toml").write_text(
        f'[[form]]\nid = "sf.{test_case.family}.probe"\nfamily = "{test_case.family}"\n'
        f'kind = "statement"\ngrammar = ["""{test_case.grammar}"""]\nuses = []\n'
        'status = "catalogued"\n'
    )
    (tmp_path / "limits.toml").write_text("")
    install()
    numbered, trees, _, _, _ = family_obligations(root=tmp_path, family=test_case.family)
    drafts: dict[tuple[str, str | None], Draft] = {
        **{(draft.kind, draft.note): draft for _, draft in numbered},
        **{(draft.kind, draft.target): draft for _, draft in numbered},
    }
    draft: Draft = drafts[(test_case.kind, test_case.label)]
    assert draft.spec is not None

    assert (
        tuple(
            case.sql
            for case in submissions(
                trees=trees[draft.form],
                style=style_of(family=test_case.family, form=draft.form),
                spec=draft.spec,
                depths=draft.depths,
            )
        )
        == test_case.expected_sql
    )
