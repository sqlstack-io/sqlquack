"""Small, fabricated capture run directories for the receipt importer."""

from __future__ import annotations

import copy
import functools
import hashlib
import json
import operator
import re
from pathlib import Path
from typing import Any

from scripts.syntax_receipts._helpers.runs import import_runs
from scripts.syntax_receipts.main.import_runs import run_import
from scripts.syntax_receipts.models import Imported
from scripts.syntax_receipts.types import Row


def sha(data: bytes) -> str:
    """The SHA-256 of `data` in hex."""
    return hashlib.sha256(data).hexdigest()


def packet_case(*, case_id: str, sql: str) -> dict[str, object]:
    """A packet row holding the exact SQL and its hash."""
    return {"id": case_id, "sql": sql, "sql_sha256": sha(sql.encode())}


def accepted_row(*, case_id: str, sql: str) -> dict[str, object]:
    """A raw record of an executed statement."""
    return {"id": case_id, "sql_sha256": sha(sql.encode()), "outcome": "accepted_executed"}


def error_row(*, case_id: str, sql: str, errno: int, message: str) -> dict[str, object]:
    """A raw record of a statement Snowflake rejected."""
    return {
        "id": case_id,
        "sql_sha256": sha(sql.encode()),
        "outcome": "error",
        "connector_fields": {"errno": errno, "sqlstate": "42000", "raw_msg": message},
    }


def write_run(
    *,
    root: Path,
    rows: list[dict[str, object]],
    packet: list[dict[str, object]],
    raw_hash: str | None = None,
    start: dict[str, object] | None = None,
    audit_events: int = 3,
) -> Path:
    """One run under `root`/runs bound to a packet under `root`/inputs; scripting by default."""
    inputs: Path = root / "inputs"
    inputs.mkdir(parents=True, exist_ok=True)
    packet_bytes: bytes = b"".join(json.dumps(row).encode() + b"\n" for row in packet)
    (inputs / "packet.jsonl").write_bytes(packet_bytes)
    run: Path = root / "runs" / "b5-test-20261103T000000Z"
    (run / "capture").mkdir(parents=True)
    raw: bytes = b"".join(json.dumps(row).encode() + b"\n" for row in rows)
    (run / "capture" / "raw.jsonl").write_bytes(raw)
    events: list[Row] = [
        {
            "event": "start",
            "packet_sha256": sha(packet_bytes),
            "parameters": {"TIMEZONE": "UTC"},
            "safety": {},
            "submission": "anonymous block, as-is",
            **(start or {}),
        },
        {"event": "recording_session", "identity_checked": True, "session_id": "S1"},
        {"event": "complete", "raw_sha256": raw_hash or sha(raw)},
    ][:audit_events]
    (run / "audit.jsonl").write_text("".join(json.dumps(e) + "\n" for e in events))
    return run


def write_reconciliation(*, run: Path, unrecorded: Row) -> Row:
    """A reconciliation record binding `run` as it stands, listing each raw line by hash."""
    audit: bytes = (run / "audit.jsonl").read_bytes()
    raw: bytes = (run / "capture" / "raw.jsonl").read_bytes()
    lines: list[bytes] = raw.splitlines()
    record: Row = {
        "format": "sqlquack-run-reconciliation/1",
        "status": "reconciled",
        "run_id": run.name,
        "audit_sha256": sha(audit),
        "packet_sha256": json.loads(audit.splitlines()[0])["packet_sha256"],
        "recording_session_id": "S1",
        "failure": {"cause": "client-side stall"},
        "observed_cleanup": {
            "drop_query_id": "q-drop",
            "drop_status": "SUCCESS",
            "dropped": "SQLQUACK_DDL_ABC123",
        },
        "inventory_check": {"counts_equal": True, "run_owned_names_remaining": []},
        "raw_records": {
            "count": len(lines),
            "file_sha256": sha(raw),
            "records": [
                {"index": index, "id": _case_id(line), "sha256": sha(line)}
                for index, line in enumerate(lines)
            ],
        },
        "unrecorded_submission": unrecorded,
    }
    return record


def unrecorded_submission(*, case_id: str, sql: str) -> Row:
    """A query-history entry for a case the run submitted but did not record."""
    return {
        "case_id": case_id,
        "case_schema": "C0002",
        "sql_sha256": sha(sql.encode()),
        "query_text_sha256": sha(sql.encode()),
        "query_id": "q-unrecorded",
        "start_time": "2026-11-04 08:18:07.366000+00:00",
        "execution_status": "FAILED_WITH_ERROR",
        "errno": 1003,
        "error_message": "SQL compilation error:\nsyntax error line 1 at position 7",
        "release_version": "10.40.100",
        "source": "query_history",
    }


def replaced(record: Row, path: tuple[str | int, ...], value: object) -> Row:
    """A deep copy of `record` with the value at `path` replaced."""
    copied: Row = copy.deepcopy(record)
    parent: Any = functools.reduce(operator.getitem, path[:-1], copied)
    parent[path[-1]] = value
    return copied


def removed(record: Row, path: tuple[str, ...]) -> Row:
    """A deep copy of `record` without the key at `path`."""
    copied: Row = copy.deepcopy(record)
    parent: Any = functools.reduce(operator.getitem, path[:-1], copied)
    del parent[path[-1]]
    return copied


def _case_id(line: bytes) -> str:
    """The case id at the start of a raw line, read even from a truncated line."""
    return re.findall(rb'^\{"id": "([^"]*)"', line)[0].decode()


def imported_run(
    *,
    root: Path,
    rows: list[dict[str, object]],
    packet: list[dict[str, object]],
    raw_hash: str | None = None,
) -> Imported:
    """The import of one fabricated run."""
    write_run(root=root, rows=rows, packet=packet, raw_hash=raw_hash)
    return import_runs(runs=root / "runs", input_dirs=[root / "inputs"])


def import_twice(
    *, root: Path, rows: list[dict[str, object]], packet: list[dict[str, object]]
) -> None:
    """Import one fabricated run twice into the same receipt and report files."""
    write_run(root=root, rows=rows, packet=packet)
    argv: list[str] = [
        str(root / "runs"),
        str(root / "out" / "r.jsonl"),
        str(root / "out" / "report.json"),
        str(root / "inputs"),
    ]
    run_import(argv)
    run_import(argv)
