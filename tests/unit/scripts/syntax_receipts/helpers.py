"""Small, fabricated capture run directories for the receipt importer."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from scripts.syntax_receipts._helpers.runs import import_runs
from scripts.syntax_receipts.main.import_runs import run_import
from scripts.syntax_receipts.models import Imported


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
) -> None:
    """One anonymous-block run under `root`/runs bound to a packet under `root`/inputs."""
    inputs: Path = root / "inputs"
    inputs.mkdir(parents=True, exist_ok=True)
    packet_bytes: bytes = b"".join(json.dumps(row).encode() + b"\n" for row in packet)
    (inputs / "packet.jsonl").write_bytes(packet_bytes)
    run: Path = root / "runs" / "b5-test-20261103T000000Z"
    (run / "capture").mkdir(parents=True)
    raw: bytes = b"".join(json.dumps(row).encode() + b"\n" for row in rows)
    (run / "capture" / "raw.jsonl").write_bytes(raw)
    events: list[dict[str, object]] = [
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
