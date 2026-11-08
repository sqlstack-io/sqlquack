"""The phase comes from a reviewed error class; an unreviewed class or a bare message fails closed."""

from __future__ import annotations

import pytest

from scripts.databricks_referee._helpers.judge import observation_of, published
from scripts.databricks_referee.constants import LATER_PHASES, UNCLASSED_LATER
from scripts.databricks_referee.exceptions import RefereeError
from scripts.databricks_referee.models import Failure, Observation
from tests.unit.scripts.databricks_referee._test_types import (
    JudgedTypeTestCase,
    PhaseCodeTestCase,
    PhaseTableTestCase,
    PhaseTestCase,
    PublishTestCase,
    RefusalTestCase,
    StatelessTestCase,
)
from tests.unit.scripts.databricks_referee.helpers import refused

PARSER_EXCERPT: str = "(line 1, pos 0)\n\n== SQL ==\nSELEC 1\n^^^\n"


@pytest.mark.parametrize(
    "test_case",
    (
        PhaseTestCase(
            description="the parser refuses a token",
            message=f"\n[PARSE_SYNTAX_ERROR] Syntax error at or near 'SELEC'. {PARSER_EXCERPT}",
            sqlstate="42601",
            expected_phase="syntax_error",
            expected_parse_only="42601",
        ),
        PhaseTestCase(
            description="the parse-tree builder refuses a MERGE with no WHEN clause",
            message=" [MERGE_WITHOUT_WHEN] There must be at least one WHEN clause in a MERGE "
            "statement. SQLSTATE: 42601\n== SQL (line 1, position 1) ==\nMERGE INTO t1\n^^^\n",
            sqlstate="42601",
            expected_phase="syntax_error",
            expected_parse_only="42601",
        ),
        PhaseTestCase(
            description="an ANSI runtime error carries the excerpt with its line and position",
            message="[DIVIDE_BY_ZERO] Division by zero. Use `try_divide` to tolerate divisor being "
            "0 and return NULL instead. SQLSTATE: 22012\n== SQL (line 1, position 8) ==\n"
            "SELECT 1 / 0\n       ^^^^^\n",
            sqlstate="22012",
            expected_phase="runtime_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="the builder refuses a feature the workspace has not enabled",
            message=" [UC_SECRETS_NOT_ENABLED] Support for Unity Catalog Secrets is not enabled. "
            "SQLSTATE: 56038\n== SQL ==\nSET TAG ON SECRET s1 k\n",
            sqlstate="56038",
            expected_phase="unsupported_feature",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="an unreviewed class carrying the excerpt stops the run",
            message="[ARITHMETIC_OVERFLOW] Overflow. SQLSTATE: 22003\n== SQL (line 1, position 8)"
            " ==\nSELECT 1\n       ^\n",
            sqlstate="22003",
            expected_phase=None,
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="analysis refuses with the syntax SQLSTATE but no parser excerpt",
            message="[MATCH_RECOGNIZE_EMPTY_MEASURES] MATCH_RECOGNIZE requires at least one "
            "expression in the MEASURES clause. SQLSTATE: 42601; line 1 pos 17",
            sqlstate="42601",
            expected_phase="semantic_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a reviewed message without an error class or SQLSTATE",
            message="QUALIFY must be used with at least one window function in SELECT or in the "
            "qualify condition.",
            sqlstate="",
            expected_phase="semantic_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a reviewed parser message without an error class, after a blank line",
            message="\n50 is not a valid byte length literal, expected syntax: DIGIT+ ('B' | 'K' | "
            "'M' | 'G').\n== SQL (line 1, position 18) ==\nSELECT * FROM t1 TABLESAMPLE ( 50 )\n",
            sqlstate="",
            expected_phase="syntax_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a reviewed parser message is not matched when the parser did not raise it",
            message="Operation not allowed: ROW FORMAT DELIMITED is only compatible with 'textfile'.",
            sqlstate="",
            expected_phase=None,
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a reviewed later-phase message is not matched in the middle of a message",
            message="The query failed. Streaming relations are only supported within CREATE "
            "STREAMING TABLE commands.",
            sqlstate="",
            expected_phase=None,
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a Unity Catalog service error decides by the class after its request id",
            message="[RequestId=0d88598f-8343-4fde-a563-a1ae0885d1ec ErrorClass=PRINCIPAL_DOES_NOT_EXIST"
            ".PRINCIPAL_DOES_NOT_EXIST] Could not find principal with name u1.",
            sqlstate="",
            expected_phase="semantic_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="analysis refuses a name",
            message="[TABLE_OR_VIEW_NOT_FOUND] The table or view `t9` cannot be found. SQLSTATE: "
            "42P01; line 1 pos 14",
            sqlstate="42P01",
            expected_phase="semantic_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a parser class with no reviewed row",
            message=f"[INVALID_SQL_SYNTAX.REPETITIVE_WINDOW_DEFINITION] x. {PARSER_EXCERPT}",
            sqlstate="42000",
            expected_phase=None,
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a reviewed parser class raised after parsing",
            message="[PARSE_SYNTAX_ERROR] Syntax error inside a string read at run time. "
            "SQLSTATE: 42601",
            sqlstate="42601",
            expected_phase=None,
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="the parse-tree builder refuses repeated keys with a non-syntax SQLSTATE",
            message="[DUPLICATE_KEY] Found duplicate keys `k`. SQLSTATE: 23505\n== SQL (line 1, "
            "position 1) ==\nALTER TABLE t1 SET TBLPROPERTIES ( 'k' = 1 , 'k' = 1 )\n^^^\n",
            sqlstate="23505",
            expected_phase="syntax_error",
            expected_parse_only="23505",
        ),
        PhaseTestCase(
            description="a parser subclass with no reviewed row, though another subclass has one",
            message="[UNSUPPORTED_FEATURE.SET_VARIABLE_USING_SET] x. SQLSTATE: 0A000"
            f"{PARSER_EXCERPT}",
            sqlstate="0A000",
            expected_phase=None,
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="an analysis class raised by the parser",
            message=f"[TABLE_OR_VIEW_NOT_FOUND] The table `t9` cannot be found. {PARSER_EXCERPT}",
            sqlstate="42P01",
            expected_phase=None,
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a volume location without a URI scheme is refused after parsing",
            message="[RequestId=0617d808-4fcf-48f5-95a3-e9fbbb67cf91 ErrorClass=INVALID_PARAMETER_"
            "VALUE.INVALID_LOCATION] storage_location does not specify a URI scheme.",
            sqlstate="",
            expected_phase="semantic_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="RESOLVE DBFS LOCATION outside a federated Hive metastore is unsupported",
            message="[UC_COMMAND_NOT_SUPPORTED.NON_HMS_FEDERATED_ENTITY] The command(s): resolve "
            "DBFS location are not supported in Unity Catalog. SQLSTATE: 0AKUC",
            sqlstate="0AKUC",
            expected_phase="unsupported_feature",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="REFRESH CATALOG without FOREIGN is the parser's refusal",
            message="\nExpected keyword 'FOREIGN'.\n== SQL (line 1, position 1) ==\n"
            "REFRESH CATALOG v1\n^^^^^^^^^^^^^^^^^^\n",
            sqlstate="",
            expected_phase="syntax_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a member user that does not exist",
            message="Users do not exist: v1",
            sqlstate="42000",
            expected_phase="semantic_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a grantee that does not exist",
            message="RESOURCE_DOES_NOT_EXIST: Could not find principal with name u1",
            sqlstate="42000",
            expected_phase="semantic_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a sharing identifier that names no metastore",
            message="INVALID_PARAMETER_VALUE: v1 is an invalid id for metastore.",
            sqlstate="42000",
            expected_phase="semantic_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="a securable type the workspace lacks, named in a grant",
            message="The securable `v1 q` of type EXTERNAL_METADATA cannot be found, and "
            "EXTERNAL_METADATA is not supported in ABAC policies. Please verify both.",
            sqlstate="42000",
            expected_phase="semantic_error",
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="an unreviewed securable type stays unmapped",
            message="The securable `v1` of type ONLINE_TABLE cannot be found, and ONLINE_TABLE is "
            "not supported in ABAC policies.",
            sqlstate="42000",
            expected_phase=None,
            expected_parse_only=None,
        ),
        PhaseTestCase(
            description="an analysis class with no reviewed row",
            message="[DIVIDE_BY_ZERO] Division by zero. SQLSTATE: 22012",
            sqlstate="22012",
            expected_phase=None,
            expected_parse_only=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_failed_reply_when_judged_then_only_a_reviewed_class_decides_the_phase(
    test_case: PhaseTestCase,
) -> None:
    found: Observation | Failure = observation_of(
        refused(message=test_case.message, sqlstate=test_case.sqlstate)
    )

    phase: str | None = getattr(found, "phase", None)
    parse_only: str | None = getattr(found, "parse_only_code", None)
    assert (phase, parse_only) == (test_case.expected_phase, test_case.expected_parse_only)


@pytest.mark.parametrize(
    "test_case",
    (
        JudgedTypeTestCase(
            description="a message naming no class and no reviewed text",
            message="WINDOW must be used with at least one window function.",
            sqlstate="",
            expected_type=Failure,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_message_without_an_error_class_when_judged_then_no_phase_is_given(
    test_case: JudgedTypeTestCase,
) -> None:
    found: Observation | Failure = observation_of(
        refused(message=test_case.message, sqlstate=test_case.sqlstate)
    )

    assert isinstance(found, test_case.expected_type)


@pytest.mark.parametrize(
    "test_case",
    (
        StatelessTestCase(
            description="a reviewed class and an unreviewed class, both without a SQLSTATE",
            reviewed_message="[NO_SUCH_STORAGE_CREDENTIAL_EXCEPTION] The credential 'v1'.",
            other_message="[TABLE_OR_VIEW_NOT_FOUND] The table `t9` cannot be found.",
            expected_phase="semantic_error",
            expected_engine_code=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_class_without_a_sqlstate_when_judged_then_only_a_reviewed_one_has_a_phase(
    test_case: StatelessTestCase,
) -> None:
    reviewed: Observation | Failure = observation_of(
        refused(message=test_case.reviewed_message, sqlstate="")
    )
    other: Observation | Failure = observation_of(
        refused(message=test_case.other_message, sqlstate="")
    )

    assert isinstance(reviewed, Observation)
    assert (reviewed.phase, reviewed.engine_code) == (
        test_case.expected_phase,
        test_case.expected_engine_code,
    )
    assert isinstance(other, Failure)


@pytest.mark.parametrize(
    "test_case",
    (
        RefusalTestCase(
            description="a failed statement with an empty message",
            message="",
            sqlstate="42601",
            expected_error="without a message",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_failure_without_a_message_when_judged_then_the_run_stops(
    test_case: RefusalTestCase,
) -> None:
    with pytest.raises(RefereeError, match=test_case.expected_error):
        observation_of(refused(message=test_case.message, sqlstate=test_case.sqlstate))


@pytest.mark.parametrize(
    "test_case",
    (
        PublishTestCase(
            description="a link and a mail address",
            messages=(
                "[X] See https://docs.example.com/a?b=1 or mail ops@example.com. SQLSTATE: 42000"
                "\nmore",
            ),
            expected_published=("[X] See <url> or mail ops at example.com. SQLSTATE: 42000",),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_message_with_a_link_and_an_address_when_published_then_both_are_masked(
    test_case: PublishTestCase,
) -> None:
    assert tuple(published(m) for m in test_case.messages) == test_case.expected_published


@pytest.mark.parametrize(
    "test_case",
    (
        PublishTestCase(
            description="a Unity Catalog service error",
            messages=("[RequestId=0d88598f-8343 ErrorClass=NOT_FOUND] Table t2 has no key.",),
            expected_published=("[RequestId=<id> ErrorClass=NOT_FOUND] Table t2 has no key.",),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_service_error_when_published_then_its_request_id_is_masked(
    test_case: PublishTestCase,
) -> None:
    assert tuple(published(m) for m in test_case.messages) == test_case.expected_published


@pytest.mark.parametrize(
    "test_case",
    (
        PublishTestCase(
            description="a table in a case schema and a bare case schema name",
            messages=(
                "[TABLE_OR_VIEW_NOT_FOUND] The table `workspace`.`sqlquack_referee_20261107t021841"
                "_979538z_1016`.`t1` cannot be found; sqlquack_referee_x is kept",
            ),
            expected_published=(
                "[TABLE_OR_VIEW_NOT_FOUND] The table `workspace`.`sqlquack_referee_<run>`.`t1` "
                "cannot be found; sqlquack_referee_<run> is kept",
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_message_naming_the_case_schema_when_published_then_its_run_stem_is_masked(
    test_case: PublishTestCase,
) -> None:
    assert tuple(published(m) for m in test_case.messages) == test_case.expected_published


@pytest.mark.parametrize(
    "test_case",
    (
        PhaseCodeTestCase(
            description="an Operation not allowed message with the excerpt",
            message="\nOperation not allowed: Values must be specified for key(s): [c1](line 1, "
            "pos 0)\n\n== SQL ==\nINSERT INTO t1 PARTITION ( c1 ) VALUES ( 1 )\n^^^\n",
            sqlstate="",
            expected_phase="syntax_error",
            expected_engine_code="PARSE",
            expected_parse_only=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_parser_message_without_a_sqlstate_when_judged_then_its_code_is_a_syntax_code(
    test_case: PhaseCodeTestCase,
) -> None:
    found: Observation | Failure = observation_of(
        refused(message=test_case.message, sqlstate=test_case.sqlstate)
    )

    assert isinstance(found, Observation)
    assert (found.phase, found.engine_code, found.parse_only_code) == (
        test_case.expected_phase,
        test_case.expected_engine_code,
        test_case.expected_parse_only,
    )


@pytest.mark.parametrize(
    "test_case",
    (
        PhaseTableTestCase(
            description="the later classes and the unclassed later messages",
            expected_absent_phase="syntax_error",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_the_later_phase_tables_when_read_then_none_gives_a_syntax_error(
    test_case: PhaseTableTestCase,
) -> None:
    later: list[str] = [*LATER_PHASES.values(), *(phase for _, phase in UNCLASSED_LATER)]

    assert test_case.expected_absent_phase not in later


@pytest.mark.parametrize(
    "test_case",
    (
        PublishTestCase(
            description="the account id, the principal and the storage bucket",
            messages=(
                "NOT_FOUND: Tag policy with name v1 not found in account "
                "0a1b2c3d-0000-4000-8000-00000000abcd",
                "PERMISSION_DENIED: Principal 'agent-7' does not have READ METADATA on Metastore "
                "'ms_eu_1'.",
                "[X] Could not access the inventory for table's storage bucket 'root-bkt-9' under "
                "'m'.",
            ),
            expected_published=(
                "NOT_FOUND: Tag policy with name v1 not found in account <account>",
                "PERMISSION_DENIED: Principal '<principal>' does not have READ METADATA on "
                "Metastore '<metastore>'.",
                "[X] Could not access the inventory for table's storage bucket '<bucket>' under "
                "'m'.",
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_messages_naming_the_account_when_published_then_its_names_are_masked(
    test_case: PublishTestCase,
) -> None:
    assert tuple(published(m) for m in test_case.messages) == test_case.expected_published


@pytest.mark.parametrize(
    "test_case",
    (
        PublishTestCase(
            description="the metastore named as a securable",
            messages=(
                "The securable `ms_eu_1` of type METASTORE cannot be found, and METASTORE is not",
            ),
            expected_published=(
                "The securable `<metastore>` of type METASTORE cannot be found, and METASTORE is "
                "not",
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_message_naming_the_metastore_securable_when_published_then_it_is_masked(
    test_case: PublishTestCase,
) -> None:
    assert tuple(published(m) for m in test_case.messages) == test_case.expected_published
