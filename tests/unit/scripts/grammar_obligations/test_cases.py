"""Syntax cases: run-database setups from their receipts, and their marks."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from scripts.grammar_obligations._helpers.cases import (
    canonical,
    digest,
    family_cases,
    run_database_setups,
    settled_cases,
)
from scripts.grammar_obligations.constants import CASE_CAPTURE_RUN_DATABASE
from scripts.grammar_obligations.models import ReceiptView
from scripts.grammar_obligations.types import Row
from tests.unit.scripts.grammar_obligations._test_types import (
    DisputedFormMarkTestCase,
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
            description="a positive the form renders, which stays a docs and engine conflict",
            sql="ALTER SEQUENCE v1 SET ;",
            expected_mark="documented",
            expected_expect="observe",
        ),
        DisputedFormMarkTestCase(
            description="a mapped case citing the form's documented positive",
            sql=MAPPED_SQL,
            expected_mark="documented",
            expected_expect="observe",
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
