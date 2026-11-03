"""The syntax receipt importer against small, fabricated run directories."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from scripts.syntax_receipts.main.import_runs import classify, import_runs, run_import


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write_run(
    root: Path, rows: list[dict], packet: list[dict], *, raw_hash: str | None = None
) -> None:
    inputs = root / "inputs"
    inputs.mkdir(parents=True, exist_ok=True)
    packet_bytes = b"".join(json.dumps(row).encode() + b"\n" for row in packet)
    (inputs / "packet.jsonl").write_bytes(packet_bytes)
    run = root / "runs" / "b5-test-20261103T000000Z"
    (run / "capture").mkdir(parents=True)
    raw = b"".join(json.dumps(row).encode() + b"\n" for row in rows)
    (run / "capture" / "raw.jsonl").write_bytes(raw)
    events = [
        {
            "event": "start",
            "packet_sha256": sha(packet_bytes),
            "parameters": {"TIMEZONE": "UTC"},
            "safety": {},
            "submission": "anonymous block, as-is",
        },
        {"event": "recording_session", "identity_checked": True},
        {"event": "complete", "raw_sha256": raw_hash or sha(raw)},
    ]
    (run / "audit.jsonl").write_text("".join(json.dumps(e) + "\n" for e in events))


def case(case_id: str, sql: str) -> dict:
    return {"id": case_id, "sql": sql, "sql_sha256": sha(sql.encode())}


def error_row(case_id: str, sql: str, errno: int, message: str) -> dict:
    return {
        "id": case_id,
        "sql_sha256": sha(sql.encode()),
        "outcome": "error",
        "connector_fields": {"errno": errno, "sqlstate": "42000", "raw_msg": message},
    }


@pytest.mark.parametrize(
    ("errno", "message", "expected"),
    (
        (1003, "SQL compilation error:\nsyntax error line 1", "syntax_error"),
        (2003, "SQL compilation error:\nObject 'T' does not exist", "semantic_error"),
        (2, "Unsupported feature 'MINUS ALL'.", "unsupported_feature"),
        (100051, "Division by zero", "runtime_error"),
        # A syntax error raised while a block evaluates an expression ran past compilation.
        (
            1003,
            "Uncaught exception of type 'EXPRESSION_ERROR' : SQL compilation error",
            "runtime_error",
        ),
        (1003, "syntax error without the compilation prefix", None),
        (None, "", None),
    ),
)
def test_given_error_fields_when_classified_then_only_established_phases_are_named(
    errno: int | None, message: str, expected: str | None
) -> None:
    result = classify(errno, message)
    assert (result[0] if result else None) == expected


def test_given_bound_rows_when_imported_then_unbindable_rows_are_refused(tmp_path: Path) -> None:
    good, other = "BEGIN RETURN 1; END;", "BEGIN RETURN 2; END;"
    rows = [
        error_row("c1", good, 1003, "SQL compilation error:\nsyntax error"),
        {"id": "c2", "sql_sha256": sha(b"something else"), "outcome": "accepted_executed"},
        {"id": "c9", "sql_sha256": sha(good.encode()), "outcome": "accepted_executed"},
        error_row("c3", other, 1003, "no prefix"),
    ]
    write_run(tmp_path, rows, [case("c1", good), case("c2", other), case("c3", other)])

    imported = import_runs(tmp_path / "runs", [tmp_path / "inputs"])

    assert [(r["case_id"], r["phase"]) for r in imported.receipts] == [("c1", "syntax_error")]
    assert sorted((r["row_id"], r["reason"].split()[0]) for r in imported.refusals) == [
        ("c2", "raw"),
        ("c3", "error"),
        ("c9", "row"),
    ]
    receipt = imported.receipts[0]
    assert receipt["sql_sha256"] == sha(good.encode())
    assert receipt["raw_record_sha256"] == sha(json.dumps(rows[0]).encode())


def test_given_raw_changed_after_audit_when_imported_then_the_whole_run_is_refused(
    tmp_path: Path,
) -> None:
    sql = "BEGIN RETURN 1; END;"
    write_run(tmp_path, [error_row("c1", sql, 1003, "x")], [case("c1", sql)], raw_hash="0" * 64)

    imported = import_runs(tmp_path / "runs", [tmp_path / "inputs"])

    assert imported.receipts == []
    assert imported.runs[0]["status"] == "refused"
    assert "differs from the audit" in imported.runs[0]["reason"]


def test_given_existing_receipts_when_imported_again_then_nothing_is_overwritten(
    tmp_path: Path,
) -> None:
    sql = "BEGIN RETURN 1; END;"
    write_run(
        tmp_path,
        [{"id": "c1", "sql_sha256": sha(sql.encode()), "outcome": "accepted_executed"}],
        [case("c1", sql)],
    )
    receipts, report = tmp_path / "out" / "r.jsonl", tmp_path / "out" / "report.json"
    run_import(tmp_path / "runs", [tmp_path / "inputs"], receipts, report)

    with pytest.raises(FileExistsError):
        run_import(tmp_path / "runs", [tmp_path / "inputs"], receipts, report)
