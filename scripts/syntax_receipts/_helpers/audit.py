"""Reading a run's audit, its raw records and the packets they are bound to."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.syntax_receipts._helpers.hashing import digest
from scripts.syntax_receipts.constants import (
    CLEANUP_SUCCESS,
    COMPLETE_EVENT,
    PACKET_SUFFIXES,
    RECONCILED_STATUS,
    RECONCILIATION_FILE,
    RECONCILIATION_FORMAT,
    RECORDING_SESSION_EVENT,
    RUN_DATABASE_KEY,
    START_EVENT,
)
from scripts.syntax_receipts.exceptions import RunRefused
from scripts.syntax_receipts.models import CaptureRun
from scripts.syntax_receipts.types import Row


def index_inputs(*, directories: list[Path]) -> dict[str, Path]:
    """Packet files under `directories` by the SHA-256 of their bytes."""
    found: dict[str, Path] = {}
    for directory in directories:
        for path in sorted(directory.rglob("*")):
            if path.is_file() and path.suffix in PACKET_SUFFIXES:
                found.setdefault(digest(path.read_bytes()), path)
    return found


def read_audit(*, run: Path) -> CaptureRun:
    """The run's audit events, refused unless they open with a start and check identity."""
    path: Path = run / "audit.jsonl"
    if not path.is_file():
        raise RunRefused("no audit.jsonl, not a capture run")
    data: bytes = path.read_bytes()
    events: list[Row] = [json.loads(line) for line in data.decode().splitlines() if line.strip()]
    if not events or events[0].get("event") != START_EVENT:
        raise RunRefused("audit does not open with a start event")
    if not any(
        e.get("event") == RECORDING_SESSION_EVENT and e.get("identity_checked") for e in events
    ):
        raise RunRefused("audit has no identity-checked recording session")
    audit_sha: str = digest(data)
    reconciliation: bytes | None = _reconciliation(run=run, events=events, audit_sha=audit_sha)
    return CaptureRun(
        path=run,
        events=events,
        audit_sha=audit_sha,
        reconciliation=None if reconciliation is None else json.loads(reconciliation),
        reconciliation_sha=None if reconciliation is None else digest(reconciliation),
    )


def _reconciliation(*, run: Path, events: list[Row], audit_sha: str) -> bytes | None:
    """The bytes of the run's reconciliation record, refused unless every check holds."""
    path: Path = run / RECONCILIATION_FILE
    if not path.is_file():
        return None
    if any(e.get("event") == COMPLETE_EVENT for e in events):
        raise RunRefused("audit has a complete event and a reconciliation record")
    if RUN_DATABASE_KEY not in events[0]:
        raise RunRefused("only a DDL run can be reconciled")
    data: bytes = path.read_bytes()
    try:
        record: object = json.loads(data)
    except ValueError as error:
        raise RunRefused("reconciliation.json is not JSON") from error
    if not isinstance(record, dict):
        raise RunRefused("reconciliation.json is not a JSON object")
    raw_path: Path = run / "capture" / "raw.jsonl"
    raw_sha: str | None = digest(raw_path.read_bytes()) if raw_path.is_file() else None
    failed: list[str] = [
        name
        for name, holds in _reconciliation_checks(
            run=run, events=events, audit_sha=audit_sha, raw_sha=raw_sha, record=record
        ).items()
        if not holds
    ]
    if failed:
        raise RunRefused(f"reconciliation does not hold: {', '.join(failed)}")
    return data


def _reconciliation_checks(
    *, run: Path, events: list[Row], audit_sha: str, raw_sha: str | None, record: Row
) -> dict[str, bool]:
    """Each named check of a reconciliation record against the run it claims to reconcile."""
    start: Row = events[0]
    sessions: list[object] = [
        e.get("session_id") for e in events if e.get("event") == RECORDING_SESSION_EVENT
    ]
    session: object = record.get("recording_session_id")
    failure: object = record.get("failure")
    cleanup: Row = _object(record.get("observed_cleanup"))
    inventory: Row = _object(record.get("inventory_check"))
    raw: Row = _object(record.get("raw_records"))
    listed: object = raw.get("records")
    entries: list[Row] = (
        listed if isinstance(listed, list) and all(isinstance(e, dict) for e in listed) else []
    )
    ids: list[object] = [entry.get("id") for entry in entries]
    return {
        "format": record.get("format") == RECONCILIATION_FORMAT,
        "status": record.get("status") == RECONCILED_STATUS,
        "run_id": record.get("run_id") == run.name,
        "audit_sha256": record.get("audit_sha256") == audit_sha,
        "packet_sha256": record.get("packet_sha256") == start.get("packet_sha256"),
        "recording_session_id": isinstance(session, str) and bool(session) and session in sessions,
        "failure": isinstance(failure, dict) and bool(failure),
        "observed_cleanup.drop_query_id": _text(cleanup.get("drop_query_id")),
        "observed_cleanup.drop_status": cleanup.get("drop_status") == CLEANUP_SUCCESS,
        "observed_cleanup.dropped": _text(cleanup.get("dropped"))
        and cleanup.get("dropped") == start.get(RUN_DATABASE_KEY),
        "inventory_check.counts_equal": inventory.get("counts_equal") is True,
        "inventory_check.run_owned_names_remaining": inventory.get("run_owned_names_remaining")
        == [],
        "raw_records.file_sha256": raw_sha is not None and raw.get("file_sha256") == raw_sha,
        "raw_records.records": bool(entries) or listed == [],
        "raw_records.count": type(raw.get("count")) is int and raw.get("count") == len(entries),
        "raw_records.index": [entry.get("index") for entry in entries] == list(range(len(entries))),
        "raw_records.id": all(_text(i) for i in ids) and len(set(ids)) == len(ids),
        "raw_records.sha256": all(_text(entry.get("sha256")) for entry in entries),
    }


def _object(value: object) -> Row:
    return value if isinstance(value, dict) else {}


def _text(value: object) -> bool:
    return isinstance(value, str) and bool(value)


def completed(*, run: CaptureRun) -> Row:
    """The audit's single complete event."""
    done: list[Row] = [e for e in run.events if e.get("event") == COMPLETE_EVENT]
    if len(done) != 1:
        raise RunRefused(f"audit has {len(done)} complete events, expected 1")
    return done[0]


def raw_lines(*, run: CaptureRun) -> list[bytes]:
    """The raw record lines, hash-checked against the complete event unless reconciled."""
    path: Path = run.path / "capture" / "raw.jsonl"
    if not path.is_file():
        raise RunRefused("no capture/raw.jsonl")
    data: bytes = path.read_bytes()
    if run.reconciliation is None and digest(data) != completed(run=run).get("raw_sha256"):
        raise RunRefused("raw.jsonl hash differs from the audit's complete event")
    return [line for line in data.split(b"\n") if line.strip()]


def packet_rows(*, index: dict[str, Path], sha: str | None) -> dict[str, Row]:
    """The rows of the packet whose bytes hash to `sha`, by id."""
    path: Path | None = index.get(sha or "")
    if path is None:
        raise RunRefused(f"packet {sha} not found among the inputs")
    rows: list[Row] = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    return {row["id"]: row for row in rows}


def error_fields(*, raw: Row) -> tuple[int | None, str | None, str]:
    """The error number, SQLSTATE and message a raw record's connector reported."""
    fields: Row = raw.get("connector_fields") or {}
    return fields.get("errno"), fields.get("sqlstate"), fields.get("raw_msg") or ""


def connector_query_id(*, raw: Row) -> str | None:
    """The query id the connector reported for a raw record."""
    return (raw.get("connector_fields") or {}).get("query_id")
