"""Snowflake operands named as numbers, objects or words, quoted values built from placeholders, and
class instance names, LIKE patterns and tags get their class and its wrong-kind negative; list
items, template alternatives and groups of options get cases that write what they name."""

from __future__ import annotations

import tomllib
from pathlib import Path

import pytest

from scripts.grammar_obligations.constants import (
    KIND_ALTERNATIVE,
    KIND_IDENTIFIER,
    KIND_NEGATIVE,
    KIND_OPERAND_CLASS,
    KIND_OPERAND_MISSING,
    KIND_OPTION_ABSENT,
    KIND_POSITIVE,
)
from scripts.syntax_obligations_snowflake.main.build import run_build
from tests.unit.scripts.syntax_obligations_snowflake._test_types import (
    DerivedLimitTestCase,
    DocumentedRefusalTestCase,
    ListItemTestCase,
    OperandClassTestCase,
)
from tests.unit.scripts.syntax_obligations_snowflake.helpers import rendered_sql

CATALOGUES: dict[str, str] = {
    "ddl": """
[[form]]
id = "sf.ddl.cancel-query"
family = "ddl"
kind = "statement"
grammar = ["CANCEL QUERY <query_id>"]
uses = []
status = "catalogued"
""",
    "transaction_session": """
[[form]]
id = "sf.transaction_session.show-transactions"
family = "transaction_session"
kind = "statement"
grammar = ["SHOW TRANSACTIONS [ LIMIT <rows> ] [ TIMEOUT = <timeout_seconds> ] [ SIZE = <max_integer> ] [ NAME = <rows_name> ] [ PURPOSE = <purpose> ] [ OWNER = <role> ] [ AGE = <days> ] [ SCALE = <fractional_seconds_precision> ] [ TAG = <purposes> ] [ PERIOD = '<num> { seconds | minutes }' ] [ PROVIDER = git_https_api ]"]
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
PERIOD: str = "SHOW TRANSACTIONS [ PERIOD = '<num> { seconds | minutes }' ]"
SCHEDULE: str = "SHOW TASKS SCHEDULE = '{ <num> MINUTE | USING CRON <expr> <time_zone> }'"
ENCRYPTION: str = "SHOW KEYS ENCRYPTION = ( { [ TYPE = 'AWS_SSE_S3' ] | [ TYPE = 'NONE' ] } )"
SET_GROUPS: str = (
    "ALTER USER [ <name> ] SET { [ [ PASSWORD = '<string>' ] [ LOGIN_NAME = <string> ] ]"
    " [ [ DISABLED = { TRUE | FALSE } ] ] }"
)


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
            description="a bare lower-case value after = is a keyword word",
            target="git_https_api in [ PROVIDER = git_https_api ]",
            expected_notes=frozenset(
                {"a valid word", "a string literal in place of git_https_api"}
            ),
        ),
        OperandClassTestCase(
            description="an operand the family fills with a quoted string is a string",
            target="<query_id>",
            expected_notes=frozenset(
                {"a valid string_literal", "an unquoted identifier in place of <query_id>"}
            ),
            family="ddl",
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
            description="an elided list continued with an empty item",
            family="transaction_session",
            grammar="DROP TAG <name> [ , ... ]",
            kind=KIND_OPERAND_MISSING,
            label="... in [ , ... ]",
            expected_sql=("DROP TAG v1 , ;",),
        ),
        ListItemTestCase(
            description="an empty elided item keeps its comma before an omitted option and )",
            family="transaction_session",
            grammar="DROP TAG ( <name> [ , ... ] [ , CASCADE ] )",
            kind=KIND_OPERAND_MISSING,
            label="... in [ , ... ]",
            expected_sql=("DROP TAG ( v1 , ) ;",),
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
    assert rendered_sql(directory=tmp_path, test_case=test_case) == test_case.expected_sql


@pytest.mark.parametrize(
    "test_case",
    (
        ListItemTestCase(
            description="a quoted template's unit alternative is written with that unit",
            family="transaction_session",
            grammar=PERIOD,
            kind=KIND_ALTERNATIVE,
            label=(
                "'<num> { seconds | minutes }' in [ PERIOD = '<num> { seconds | minutes }' ]"
                " → minutes"
            ),
            expected_sql=("SHOW TRANSACTIONS PERIOD = '60 minutes' ;",),
        ),
        ListItemTestCase(
            description="a template made of one choice writes its CRON alternative",
            family="transaction_session",
            grammar=SCHEDULE,
            kind=KIND_ALTERNATIVE,
            label=(
                "'{ <num> MINUTE | USING CRON <expr> <time_zone> }' → USING CRON <expr> <time_zone>"
            ),
            expected_sql=("SHOW TASKS SCHEDULE = 'USING CRON 0 9 * * * UTC' ;",),
        ),
        ListItemTestCase(
            description="an alternative made only of options writes its option",
            family="transaction_session",
            grammar=ENCRYPTION,
            kind=KIND_ALTERNATIVE,
            label="{ [ TYPE = 'AWS_SSE_S3' ] | [ TYPE = 'NONE' ] } → [ TYPE = 'NONE' ]",
            expected_sql=("SHOW KEYS ENCRYPTION = ( TYPE = 'NONE' ) ;",),
        ),
        ListItemTestCase(
            description="an absent option of a one-option alternative still empties it",
            family="transaction_session",
            grammar=ENCRYPTION,
            kind=KIND_OPTION_ABSENT,
            label="[ TYPE = 'AWS_SSE_S3' ]",
            expected_sql=("SHOW KEYS ENCRYPTION = ( ) ;",),
        ),
        ListItemTestCase(
            description="a SET list of option groups writes one option of its first group",
            family="security_admin",
            grammar=SET_GROUPS,
            kind=KIND_POSITIVE,
            label="the documented form with every required part and no optional part",
            expected_sql=("ALTER USER SET PASSWORD = 'm' ;",),
        ),
        ListItemTestCase(
            description="an absent first option leaves its group's next option written",
            family="security_admin",
            grammar=SET_GROUPS,
            kind=KIND_OPTION_ABSENT,
            label="[ PASSWORD = '<string>' ]",
            expected_sql=("ALTER USER SET LOGIN_NAME = 'm' ;",),
        ),
        ListItemTestCase(
            description="a reserved name, bare and quoted, reaches a SET list that writes an option",
            family="security_admin",
            grammar=SET_GROUPS,
            kind=KIND_IDENTIFIER,
            label="<name>",
            expected_sql=(
                "ALTER USER SELECT SET PASSWORD = 'm' ;",
                "ALTER USER \"SELECT\" SET PASSWORD = 'm' ;",
            ),
        ),
        ListItemTestCase(
            description="a documented number is one token, not one digit after another",
            family="ddl",
            grammar="ALTER POSTGRES INSTANCE <name> SET POSTGRES_VERSION = { 16 | 17 }",
            kind=KIND_POSITIVE,
            label="the documented form with every required part and no optional part",
            expected_sql=("ALTER POSTGRES INSTANCE v1 SET POSTGRES_VERSION = 16 ;",),
        ),
        ListItemTestCase(
            description="an upper-case keyword with a lower-case part is written, not filled",
            family="ddl",
            grammar="ALTER WAREHOUSE <name> SET RESOURCE_CONSTRAINT = { MEMORY_1X | MEMORY_1X_x86 }",
            kind=KIND_ALTERNATIVE,
            label="{ MEMORY_1X | MEMORY_1X_x86 } → MEMORY_1X_x86",
            expected_sql=("ALTER WAREHOUSE v1 SET RESOURCE_CONSTRAINT = MEMORY_1X_x86 ;",),
        ),
        ListItemTestCase(
            description="an upper-case name fill is quoted with mixed case",
            family="security_admin",
            grammar="SHOW INTEGRATIONS OF TYPE <integration_type_name>",
            kind=KIND_IDENTIFIER,
            label="quoted: a double-quoted identifier with blanks and mixed case",
            expected_sql=('SHOW INTEGRATIONS OF TYPE "security integrations Q" ;',),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_generator_rule_when_building_then_its_case_writes_what_it_names(
    tmp_path: Path, test_case: ListItemTestCase
) -> None:
    assert rendered_sql(directory=tmp_path, test_case=test_case) == test_case.expected_sql
