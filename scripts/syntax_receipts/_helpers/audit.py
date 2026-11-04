"""Reading a run's audit, its raw records and the packets they are bound to."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.syntax_receipts._helpers.hashing import digest
from scripts.syntax_receipts.constants import (
    CLEANUP_EVIDENCE,
    COMPLETE_EVENT,
    PACKET_SUFFIXES,
    RECONCILED_STATUS,
    RECONCILIATION_EVIDENCE,
    RECONCILIATION_FILE,
    RECONCILIATION_FORMAT,
    RECORDING_SESSION_EVENT,
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
    return CaptureRun(
        path=run,
        events=events,
        audit_sha=audit_sha,
        reconciliation=_reconciliation(run=run, events=events, audit_sha=audit_sha),
    )


def _reconciliation(*, run: Path, events: list[Row], audit_sha: str) -> Row | None:
    """The run's reconciliation record, refused unless it stands in for a missing complete event."""
    path: Path = run / RECONCILIATION_FILE
    if not path.is_file():
        return None
    if any(e.get("event") == COMPLETE_EVENT for e in events):
        raise RunRefused("audit has a complete event and a reconciliation record")
    record: Row = json.loads(path.read_bytes())
    sessions: list[object] = [
        e.get("session_id") for e in events if e.get("event") == RECORDING_SESSION_EVENT
    ]
    bindings: dict[str, bool] = {
        "format": record.get("format") == RECONCILIATION_FORMAT,
        "status": record.get("status") == RECONCILED_STATUS,
        "run_id": record.get("run_id") == run.name,
        "audit_sha256": record.get("audit_sha256") == audit_sha,
        "packet_sha256": record.get("packet_sha256") == events[0].get("packet_sha256"),
        "recording_session_id": bool(record.get("recording_session_id"))
        and record.get("recording_session_id") in sessions,
    }
    unbound: list[str] = [key for key, bound in bindings.items() if not bound]
    if unbound:
        raise RunRefused(f"reconciliation does not bind this run: {', '.join(unbound)}")
    cleanup: Row = record.get("observed_cleanup") or {}
    missing: list[str] = [key for key in RECONCILIATION_EVIDENCE if not record.get(key)] + [
        f"observed_cleanup.{key}" for key in CLEANUP_EVIDENCE if not cleanup.get(key)
    ]
    if missing:
        raise RunRefused(f"reconciliation has no {', '.join(missing)}")
    raw: object = record["raw_records"]
    listed: list[Row] = (raw.get("records") or []) if isinstance(raw, dict) else []
    if (
        not isinstance(raw, dict)
        or raw.get("count") != len(listed)
        or [entry.get("index") for entry in listed] != list(range(len(listed)))
    ):
        raise RunRefused("reconciliation raw records are not counted and indexed in order")
    return record


def completed(*, run: CaptureRun) -> Row:
    """The audit's single complete event."""
    done: list[Row] = [e for e in run.events if e.get("event") == COMPLETE_EVENT]
    if len(done) != 1:
        raise RunRefused(f"audit has {len(done)} complete events, expected 1")
    return done[0]


def raw_lines(*, run: CaptureRun) -> list[bytes]:
    """The raw record lines, refused unless they hash to the complete event's raw hash.

    A reconciled run has no complete event; its lines are checked one by one against the
    reconciliation instead.
    """
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
