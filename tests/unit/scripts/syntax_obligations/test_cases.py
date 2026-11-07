"""Run-database setups bound to syntax cases from their receipts."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from scripts.syntax_obligations._helpers.cases import canonical, digest, run_database_setups
from scripts.syntax_obligations.constants import CASE_CAPTURE_RUN_DATABASE
from tests.unit.scripts.syntax_obligations._test_types import SetupBindingTestCase

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
