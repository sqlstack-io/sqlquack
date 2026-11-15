"""Syntax cases: run-database setups from their receipts, their marks, and expectations their
receipts disprove."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from scripts.grammar_obligations._helpers.cases import (
    canonical,
    digest,
    family_cases,
    receipt_phases,
    receipt_settlements,
    run_database_setups,
    settled_cases,
)
from scripts.grammar_obligations._helpers.obligations import family_obligations
from scripts.grammar_obligations.constants import (
    CASE_CAPTURE_RUN_DATABASE,
    KIND_INTERACTION_ALL,
    KIND_OPTION_PRESENT,
)
from scripts.grammar_obligations.models import Draft, ReceiptView
from scripts.grammar_obligations.types import Row
from tests.unit.scripts.grammar_obligations._test_types import (
    DisprovedExpectationTestCase,
    DisputedFormMarkTestCase,
    PackedRefusalTestCase,
    ProbeSettlementTestCase,
    SetupBindingTestCase,
)

RUN_SETUP: list[str] = [
    "CREATE DATABASE <run_database> COMMENT = 'sqlquack ddl run <run_nonce>'",
    "USE SCHEMA <run_database>.<case_schema>",
]


@pytest.mark.parametrize(
    "test_case",
    (
        SetupBindingTestCase(
            description="the report lists the setup under another statement's SQL",
            receipt_sql_sha256="case-sql",
            report_input_sql_sha256="other-sql",
            expected_setup=tuple(RUN_SETUP),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_receipted_setup_when_binding_then_the_case_keeps_the_setup_its_hash_names(
    tmp_path: Path, test_case: SetupBindingTestCase
) -> None:
    report: dict[str, object] = {
        "inputs": [{"sql_sha256": test_case.report_input_sql_sha256, "setup": RUN_SETUP}],
        "runs": [{"run_id": "run-1", "status": "imported"}],
    }
    (tmp_path / "import-1.report.json").write_text(json.dumps(report))
    receipt: dict[str, str] = {
        "case_id": "sf.functions.f.kcase",
        "sql_sha256": test_case.receipt_sql_sha256,
        "setup_sha256": digest(canonical(RUN_SETUP)),
        "capture_method": CASE_CAPTURE_RUN_DATABASE,
        "run_id": "run-1",
        "phase": "syntax_error",
        "recorded_at": "2026-11-04T03:53:19+00:00",
    }
    (tmp_path / "import-1.jsonl").write_text(json.dumps(receipt) + "\n")

    setups, flagged = run_database_setups(tmp_path)

    assert setups == {
        ("sf.functions.f.kcase", test_case.receipt_sql_sha256): list(test_case.expected_setup)
    }
    assert flagged == []


DISPUTED_FORM: str = "sf.ddl.alter-sequence.sequence"

DISPUTED_CATALOGUE: str = f"""
[[form]]
id = "{DISPUTED_FORM}"
family = "ddl"
source = "doc.sql.alter-sequence"
anchor = "Syntax"
kind = "statement"
grammar = ["ALTER SEQUENCE <name> [ SET ] [ COMMENT = '<string_literal>' ]"]
uses = []
status = "catalogued"
"""

MAPPED_SQL: str = "ALTER SEQUENCE IF EXISTS v1 ;"

DISPUTED_REASON: str = "the page documents this SQL, but Snowflake rejects it as a syntax error"

RESERVED_NOTE: str = (
    "reserved: a reserved keyword, unquoted (expected rejection) and double-quoted"
    " (expected acceptance)"
)

DISPUTED_LIMITS: str = f"""
[[suppress]]
form = "{DISPUTED_FORM}"
kind = "positive"
note = "the documented form with every required part and no optional part"
reason = "{DISPUTED_REASON}"

[[suppress]]
form = "{DISPUTED_FORM}"
kind = "operand_type_class"
target = "<name>"
reason = "{DISPUTED_REASON}"

[[suppress]]
form = "{DISPUTED_FORM}"
kind = "option_absent"
target = "[ SET ]"
reason = "{DISPUTED_REASON}"

[[suppress]]
form = "{DISPUTED_FORM}"
kind = "option_absent"
target = "[ COMMENT = '<string_literal>' ]"
reason = "{DISPUTED_REASON}"

[[suppress]]
form = "{DISPUTED_FORM}"
kind = "identifier_class"
target = "<name>"
note = "{RESERVED_NOTE}"
reason = "{DISPUTED_REASON}"

[[limit]]
form = "{DISPUTED_FORM}"
kind = "positive"
note = "the documented form, observed"
sql = "ALTER SEQUENCE v1 ;"
expect = "observe"
documented = true
context = "raw"

[[limit]]
form = "{DISPUTED_FORM}"
kind = "identifier_class"
target = "<name>"
note = "{RESERVED_NOTE}, observed"
sql = 'ALTER SEQUENCE "SELECT" ;'
expect = "observe"
documented = true
also_sql = "ALTER SEQUENCE SELECT ;"
also_expect = "observe"
context = "raw"
"""


@pytest.mark.parametrize(
    "test_case",
    (
        DisputedFormMarkTestCase(
            description="a positive the form renders, which its receipts settle and which stays documented",
            sql="ALTER SEQUENCE v1 SET ;",
            expected_mark="documented",
            expected_expect="reject",
        ),
        DisputedFormMarkTestCase(
            description="a mapped case citing the form's documented positive, settled by its receipts",
            sql=MAPPED_SQL,
            expected_mark="documented",
            expected_expect="reject",
        ),
        DisputedFormMarkTestCase(
            description="a rejection the form renders observed, which its receipt settles",
            sql="ALTER SEQUENCE v1 COMMENT = m1 ;",
            expected_mark="settled reject",
            expected_expect="reject",
        ),
        DisputedFormMarkTestCase(
            description="a hand-written intended rejection, which its receipt settles",
            sql="ALTER SEQUENCE SELECT ;",
            expected_mark="settled reject",
            expected_expect="reject",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_form_snowflake_refuses_when_marking_then_only_its_positives_are_documented(
    tmp_path: Path, test_case: DisputedFormMarkTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / "ddl.toml").write_text(DISPUTED_CATALOGUE)
    (tmp_path / "limits.toml").write_text(DISPUTED_LIMITS)
    generated: list[Row] = family_cases(
        root=tmp_path, evidence=tmp_path, family="ddl", setups={}, held=frozenset()
    )[0]
    positive: Row = {case["sql"]: case for case in generated}["ALTER SEQUENCE v1 ;"]
    mapped: Row = {
        **positive,
        "case_id": f"{DISPUTED_FORM}.kmapped",
        "sql": MAPPED_SQL,
        "sql_sha256": digest(MAPPED_SQL),
    }
    (tmp_path / "mapped-cases").mkdir()
    (tmp_path / "mapped-cases" / "ddl.jsonl").write_text(canonical(mapped) + "\n")
    generated.append(mapped)
    refused: ReceiptView = ReceiptView(
        phases={(case["case_id"], case["sql_sha256"]): "syntax_error" for case in generated},
        settlements={
            (
                case["case_id"],
                case["sql_sha256"],
                case["setup_sha256"],
                case["session_sha256"],
            ): ("reject", "")
            for case in generated
        },
    )

    settled, _, marked = settled_cases(
        root=tmp_path,
        evidence=tmp_path,
        family="ddl",
        receipts=refused,
        setups={},
        held=frozenset(),
    )

    case: Row = {case["sql"]: case for case in settled}[test_case.sql]
    marks: dict[str, str] = {row["case_id"]: row["mark"] for row in marked}
    assert marks.get(case["case_id"]) == test_case.expected_mark
    assert case["expect"] == test_case.expected_expect


DROP_TAG_CATALOGUE: str = """
[[form]]
id = "sf.ddl.drop-tag"
family = "ddl"
kind = "statement"
grammar = ["DROP TAG <name>"]
uses = []
status = "catalogued"
"""


@pytest.mark.parametrize(
    "test_case",
    (
        DisprovedExpectationTestCase(
            description="an accept refused with a syntax error is decided by its receipts",
            sql="DROP TAG v1 ;",
            settlement="reject",
            phase="syntax_error",
            expected_expect="reject",
            expected_mark="settled reject",
        ),
        DisprovedExpectationTestCase(
            description="an accept refused after parsing keeps its expectation",
            sql="DROP TAG v1 ;",
            settlement="reject",
            phase="semantic_error",
            expected_expect="accept",
            expected_mark=None,
        ),
        DisprovedExpectationTestCase(
            description="a reject Snowflake parses is decided by its receipts",
            sql="DROP TAG ;",
            settlement="accept",
            phase="executed",
            expected_expect="accept",
            expected_mark="settled accept",
        ),
        DisprovedExpectationTestCase(
            description="a reject its receipts confirm keeps its expectation",
            sql="DROP TAG ;",
            settlement="reject",
            phase="syntax_error",
            expected_expect="reject",
            expected_mark=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_receipts_against_a_generated_expectation_when_settling_then_receipts_decide(
    tmp_path: Path, test_case: DisprovedExpectationTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / "ddl.toml").write_text(DROP_TAG_CATALOGUE)
    (tmp_path / "limits.toml").write_text("")
    generated: list[Row] = family_cases(
        root=tmp_path, evidence=tmp_path, family="ddl", setups={}, held=frozenset()
    )[0]
    case: Row = {case["sql"]: case for case in generated}[test_case.sql]
    receipts: ReceiptView = ReceiptView(
        phases={(case["case_id"], case["sql_sha256"]): test_case.phase},
        settlements={
            (
                case["case_id"],
                case["sql_sha256"],
                case["setup_sha256"],
                case["session_sha256"],
            ): (test_case.settlement, "")
        },
    )

    settled, _, marked = settled_cases(
        root=tmp_path,
        evidence=tmp_path,
        family="ddl",
        receipts=receipts,
        setups={},
        held=frozenset(),
    )

    found: Row = {row["sql"]: row for row in settled}[test_case.sql]
    marks: dict[str, str] = {row["case_id"]: row["mark"] for row in marked}
    assert (found["expect"], marks.get(found["case_id"])) == (
        test_case.expected_expect,
        test_case.expected_mark,
    )


PROBE_SETUP: str = digest(canonical([]))

PROBE_PARSED: Row = {"phase": "semantic_error", "errno": 2105, "no_effect": "probe_role"}


@pytest.mark.parametrize(
    "test_case",
    (
        ProbeSettlementTestCase(
            description="a probe-role refusal after parsing settles a reject under any setup",
            receipts=({**PROBE_PARSED, "setup_sha256": PROBE_SETUP},),
            expected_expect="accept",
            expected_mark="settled accept",
        ),
        ProbeSettlementTestCase(
            description="the same refusal from another setup without the probe role settles nothing",
            receipts=({"phase": "semantic_error", "errno": 2105, "setup_sha256": PROBE_SETUP},),
            expected_expect="reject",
            expected_mark=None,
        ),
        ProbeSettlementTestCase(
            description="a probe-role refusal against a syntax error under the case's setup settles nothing",
            receipts=(
                {**PROBE_PARSED, "setup_sha256": PROBE_SETUP},
                {"phase": "syntax_error", "errno": 1003, "setup_sha256": None},
            ),
            expected_expect="reject",
            expected_mark=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_probe_role_receipts_when_settling_then_they_bind_any_setup(
    tmp_path: Path, test_case: ProbeSettlementTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / "ddl.toml").write_text(DROP_TAG_CATALOGUE)
    (tmp_path / "limits.toml").write_text("")
    generated: list[Row] = family_cases(
        root=tmp_path, evidence=tmp_path, family="ddl", setups={}, held=frozenset()
    )[0]
    case: Row = {case["sql"]: case for case in generated}["DROP TAG ;"]
    assert case["setup_sha256"] != PROBE_SETUP
    receipts: Path = tmp_path / "receipts"
    receipts.mkdir()
    (receipts / "ddl.jsonl").write_text(
        "".join(
            canonical(
                {
                    "case_id": case["case_id"],
                    "sql_sha256": case["sql_sha256"],
                    "session_sha256": case["session_sha256"],
                    **receipt,
                    "setup_sha256": receipt["setup_sha256"] or case["setup_sha256"],
                }
            )
            + "\n"
            for receipt in test_case.receipts
        )
    )

    settled, _, marked = settled_cases(
        root=tmp_path,
        evidence=tmp_path,
        family="ddl",
        receipts=ReceiptView(
            phases=receipt_phases(receipts), settlements=receipt_settlements(receipts)
        ),
        setups={},
        held=frozenset(),
    )

    found: Row = {row["sql"]: row for row in settled}["DROP TAG ;"]
    marks: dict[str, str] = {row["case_id"]: row["mark"] for row in marked}
    assert (found["expect"], marks.get(found["case_id"])) == (
        test_case.expected_expect,
        test_case.expected_mark,
    )


ALTER_WIDGET_CATALOGUE: str = """
[[form]]
id = "sf.ddl.alter-widget.set"
family = "ddl"
kind = "statement"
grammar = ["ALTER WIDGET <name> SET [ COMMENT = '<string_literal>' ] [ MAX_ROWS = <integer> ]"]
uses = []
status = "catalogued"
"""
ALL_OPTIONS_SQL: str = "ALTER WIDGET v1 SET COMMENT = 'm' MAX_ROWS = 1 ;"


@pytest.mark.parametrize(
    "test_case",
    (
        PackedRefusalTestCase(
            description="a syntax error on the case with every option gives each option its own case",
            refused_sql=ALL_OPTIONS_SQL,
            phase="syntax_error",
            expected_option_sql=(
                "ALTER WIDGET v1 SET COMMENT = 'm' ;",
                "ALTER WIDGET v1 SET MAX_ROWS = 1 ;",
            ),
        ),
        PackedRefusalTestCase(
            description="an error after parsing on the case with every option keeps the options together",
            refused_sql=ALL_OPTIONS_SQL,
            phase="semantic_error",
            expected_option_sql=(ALL_OPTIONS_SQL, ALL_OPTIONS_SQL),
        ),
        PackedRefusalTestCase(
            description="a syntax error on a case that covers no option alone keeps the options together",
            refused_sql="ALTER WIDGET v1 SET COMMENT = 'm' ;",
            phase="syntax_error",
            expected_option_sql=(ALL_OPTIONS_SQL, ALL_OPTIONS_SQL),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_refused_case_with_every_option_when_building_cases_then_each_option_gets_its_own(
    tmp_path: Path, test_case: PackedRefusalTestCase
) -> None:
    (tmp_path / "catalogue").mkdir()
    (tmp_path / "catalogue" / "ddl.toml").write_text(ALTER_WIDGET_CATALOGUE)
    (tmp_path / "limits.toml").write_text("")
    packed: list[Row] = family_cases(
        root=tmp_path, evidence=tmp_path, family="ddl", setups={}, held=frozenset()
    )[0]
    refused: Row = {case["sql"]: case for case in packed}[test_case.refused_sql]

    cases: list[Row] = family_cases(
        root=tmp_path,
        evidence=tmp_path,
        family="ddl",
        setups={},
        held=frozenset(),
        phases={(refused["case_id"], refused["sql_sha256"]): test_case.phase},
    )[0]

    by_obligation: dict[str, str] = {}
    for case in cases:
        by_obligation |= dict.fromkeys(case["obligations"], str(case["sql"]))
    all_options: Row = {case["sql"]: case for case in cases}[ALL_OPTIONS_SQL]
    numbered: list[tuple[str, Draft]] = family_obligations(root=tmp_path, family="ddl")[0]
    by_kind: dict[str, list[str]] = {}
    for found, draft in numbered:
        by_kind.setdefault(draft.kind, []).append(found)
    assert (
        by_obligation[by_kind[KIND_OPTION_PRESENT][0]],
        by_obligation[by_kind[KIND_OPTION_PRESENT][1]],
        by_kind[KIND_INTERACTION_ALL][0] in all_options["obligations"],
    ) == (*test_case.expected_option_sql, True), test_case.description
