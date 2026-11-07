"""Run-database setups read back from counted receipts."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from scripts.syntax_obligations._helpers.cases import canonical, digest, run_database_setups
from tests.unit.scripts.syntax_obligations._test_types import RecordedSetupTestCase

SETUP: list[str] = [
    "CREATE DATABASE <run_database> COMMENT = 'sqlquack ddl run <run_nonce>'",
    "USE SCHEMA <run_database>.CASES",
]
CASE_ID: str = "sf.expressions.data-types-uuid.k0a320b3136f"
SQL_SHA: str = "0a320b3136f" + "0" * 53


@pytest.mark.parametrize(
    "test_case",
    (
        RecordedSetupTestCase(
            description="the setup is listed as an input of the receipt's own SQL",
            input_sql_sha256=SQL_SHA,
            expected_setups={(CASE_ID, SQL_SHA): SETUP},
        ),
        RecordedSetupTestCase(
            description="the setup is listed only as an input of another SQL",
            input_sql_sha256="f" * 64,
            expected_setups={(CASE_ID, SQL_SHA): SETUP},
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_counted_run_database_receipt_when_read_then_its_setup_is_found_by_digest(
    test_case: RecordedSetupTestCase, tmp_path: Path
) -> None:
    report: dict[str, object] = {
        "inputs": [{"sql_sha256": test_case.input_sql_sha256, "setup": SETUP}],
        "runs": [{"run_id": "run-1", "status": "imported"}],
    }
    receipt: dict[str, object] = {
        "capture_method": "execute_ddl_in_run_database",
        "case_id": CASE_ID,
        "phase": "accepted",
        "recorded_at": "2026-11-04T02:40:54+00:00",
        "run_id": "run-1",
        "setup_sha256": digest(canonical(SETUP)),
        "sql_sha256": SQL_SHA,
    }
    (tmp_path / "import-1.report.json").write_text(json.dumps(report))
    (tmp_path / "import-1.jsonl").write_text(json.dumps(receipt) + "\n")

    assert run_database_setups(tmp_path) == (test_case.expected_setups, [])
