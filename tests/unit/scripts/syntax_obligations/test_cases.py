"""Run-database setups bound to syntax cases from their receipts."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.syntax_obligations._helpers.cases import canonical, digest, run_database_setups
from scripts.syntax_obligations.constants import CASE_CAPTURE_RUN_DATABASE

RUN_SETUP: list[str] = [
    "CREATE DATABASE <run_database> COMMENT = 'sqlquack ddl run <run_nonce>'",
    "USE SCHEMA <run_database>.<case_schema>",
]


def test_given_a_receipt_whose_setup_text_is_listed_for_other_sql_when_binding_then_the_case_keeps_that_setup(
    tmp_path: Path,
) -> None:
    (tmp_path / "import-1.report.json").write_text(
        json.dumps(
            {
                "inputs": [{"sql_sha256": "other-sql", "setup": RUN_SETUP}],
                "runs": [{"run_id": "run-1", "status": "imported"}],
            }
        )
    )
    receipt: dict[str, str] = {
        "case_id": "sf.functions.f.kcase",
        "sql_sha256": "case-sql",
        "setup_sha256": digest(canonical(RUN_SETUP)),
        "capture_method": CASE_CAPTURE_RUN_DATABASE,
        "run_id": "run-1",
        "phase": "syntax_error",
        "recorded_at": "2026-11-04T03:53:19+00:00",
    }
    (tmp_path / "import-1.jsonl").write_text(json.dumps(receipt) + "\n")

    setups, flagged = run_database_setups(tmp_path)

    assert setups == {("sf.functions.f.kcase", "case-sql"): RUN_SETUP}
    assert flagged == []
