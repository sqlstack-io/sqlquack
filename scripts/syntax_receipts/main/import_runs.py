"""Import private Snowflake capture runs as normalized syntax-coverage receipts.

Each run directory holds an append-only ``audit.jsonl`` whose ``start`` event binds the packet and
session parameters and whose ``complete`` event binds the raw records. A row becomes a receipt only
when its exact SQL hash, packet input, session parameters and error phase are all established;
every other row is listed as refused with its reason. Raw messages and row values never leave the
private records: a receipt carries their hashes, the error number and SQLSTATE, and the phase.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from scripts.syntax_receipts.constants import (
    CAPTURE_CANONICAL,
    CAPTURE_EXECUTE,
    CAPTURE_EXPLAIN,
    COMPILATION_PREFIX,
    CONTROLLER_CATEGORIES,
    EXPLAIN_METHOD,
    RECEIPT_FORMAT,
    RUNTIME_ERRNO_FLOOR,
    SCRIPTING_SUBMISSION,
    SYNTAX_ERRNO,
    UNCAUGHT_PREFIX,
    UNSUPPORTED_ERRNO,
    UNSUPPORTED_PREFIX,
)

type Row = dict[str, Any]


@dataclass
class Imported:
    receipts: list[Row] = field(default_factory=list)
    inputs: dict[str, Row] = field(default_factory=dict)
    refusals: list[Row] = field(default_factory=list)
    runs: list[Row] = field(default_factory=list)


class RunRefused(Exception):
    """The whole run cannot be imported."""


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_json(value: object) -> bytes:
    """The bytes every session and setup hash is taken over."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def classify(errno: int | None, message: str) -> tuple[str, str] | None:
    """The phase of a Snowflake error, or None when the recorded fields do not establish one."""
    if message.startswith(UNCAUGHT_PREFIX):
        return "runtime_error", "uncaught exception raised while the block ran"
    if errno == UNSUPPORTED_ERRNO and message.startswith(UNSUPPORTED_PREFIX):
        return "unsupported_feature", "errno 2 Unsupported feature"
    if errno is not None and errno >= RUNTIME_ERRNO_FLOOR:
        return "runtime_error", f"errno {errno} is an execution-time code"
    if errno == SYNTAX_ERRNO and message.startswith(COMPILATION_PREFIX):
        return "syntax_error", "errno 1003 SQL compilation syntax error"
    if errno is not None and message.startswith(COMPILATION_PREFIX):
        return "semantic_error", f"errno {errno} compilation error after parsing"
    return None


def index_inputs(directories: list[Path]) -> dict[str, Path]:
    """Packet files under ``directories`` by the SHA-256 of their bytes."""
    found: dict[str, Path] = {}
    for directory in directories:
        for path in sorted(directory.rglob("*")):
            if path.is_file() and path.suffix in (".jsonl", ".json"):
                found.setdefault(digest(path.read_bytes()), path)
    return found


def read_audit(run: Path) -> tuple[list[Row], str]:
    path = run / "audit.jsonl"
    if not path.is_file():
        raise RunRefused("no audit.jsonl, not a capture run")
    data = path.read_bytes()
    events = [json.loads(line) for line in data.decode().splitlines() if line.strip()]
    if not events or events[0].get("event") != "start":
        raise RunRefused("audit does not open with a start event")
    if not any(e.get("event") == "recording_session" and e.get("identity_checked") for e in events):
        raise RunRefused("audit has no identity-checked recording session")
    return events, digest(data)


def completed(events: list[Row]) -> Row:
    done = [e for e in events if e.get("event") == "complete"]
    if len(done) != 1:
        raise RunRefused(f"audit has {len(done)} complete events, expected 1")
    return done[0]


def raw_lines(run: Path, events: list[Row]) -> list[bytes]:
    path = run / "capture" / "raw.jsonl"
    if not path.is_file():
        raise RunRefused("no capture/raw.jsonl")
    data = path.read_bytes()
    if digest(data) != completed(events).get("raw_sha256"):
        raise RunRefused("raw.jsonl hash differs from the audit's complete event")
    return [line for line in data.split(b"\n") if line.strip()]


def packet_rows(index: dict[str, Path], sha: str | None) -> dict[str, Row]:
    path = index.get(sha or "")
    if path is None:
        raise RunRefused(f"packet {sha} not found among the inputs")
    rows = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    return {row["id"]: row for row in rows}


def error_fields(raw: Row) -> tuple[int | None, str | None, str]:
    fields = raw.get("connector_fields") or {}
    return fields.get("errno"), fields.get("sqlstate"), fields.get("raw_msg") or ""


@dataclass(frozen=True)
class Observation:
    """One recorded statement, bound to its run, packet, exact SQL, setup and session."""

    run_id: str
    audit_sha: str
    raw_bytes: bytes
    case_id: str
    packet_sha: str
    sql_sha: str
    setup: list[str]
    session: Row
    method: str
    accepted: bool
    errno: int | None
    sqlstate: str | None
    message: str


def receipt(observation: Observation, **extra: Any) -> Row:
    o = observation
    if o.accepted:
        phase, basis = "accepted", "statement compiled" if o.method == CAPTURE_EXPLAIN else "ran"
    else:
        classified = classify(o.errno, o.message)
        if classified is None:
            raise ValueError(f"error phase not established (errno {o.errno})")
        phase, basis = classified
    return {
        "format": RECEIPT_FORMAT,
        "case_id": o.case_id,
        "packet_sha256": o.packet_sha,
        "sql_sha256": o.sql_sha,
        "setup_sha256": digest(canonical_json(o.setup)),
        "session_sha256": digest(canonical_json(o.session)),
        "session_parameters": o.session,
        "run_id": o.run_id,
        "capture_method": o.method,
        "outcome": "accepted" if o.accepted else "error",
        "errno": o.errno,
        "sqlstate": o.sqlstate,
        "phase": phase,
        "phase_basis": basis,
        "raw_record_sha256": digest(o.raw_bytes),
        "run_audit_sha256": o.audit_sha,
        **extra,
    }


def add_input(imported: Imported, sql: str, setup: list[str]) -> str:
    sha = digest(sql.encode())
    imported.inputs.setdefault(sha, {"sql_sha256": sha, "sql": sql, "setup": setup})
    return sha


def import_packet_run(
    imported: Imported, run: Path, events: list[Row], audit_sha: str, index: dict[str, Path]
) -> int:
    """Anonymous-block and compile-only EXPLAIN runs: one raw line per packet row or statement."""
    start = events[0]
    explain = start.get("method") == EXPLAIN_METHOD
    session = {**start.get("parameters", {}), **start.get("safety", {})}
    if not start.get("parameters"):
        raise RunRefused("start event records no session parameters")
    packet = packet_rows(index, start.get("packet_sha256"))
    count = 0
    for line in raw_lines(run, events):
        raw = json.loads(line)
        row_id = raw.get("id", "?")
        case = packet.get(row_id)
        if case is None or "sql" not in case:
            refuse(imported, run.name, row_id, "row id is not in the bound packet")
            continue
        sql_sha = raw.get("sql_sha256")
        if (
            not sql_sha
            or sql_sha != case.get("sql_sha256")
            or digest(case["sql"].encode()) != sql_sha
        ):
            refuse(imported, run.name, row_id, "raw SQL hash does not match the packet's exact SQL")
            continue
        add_input(imported, case["sql"], [])
        errno, sqlstate, message = error_fields(raw)
        extra: Row = {
            "query_id": raw.get("query_id") or (raw.get("connector_fields") or {}).get("query_id"),
            "warehouse_version": raw.get("warehouse_version"),
            "recorded_at": raw.get("started_at"),
        }
        if explain:
            extra["statement_index"] = raw.get("statement_index")
            extra["statements_in_input"] = case.get("statements_in_input")
            extra["input_sha256"] = case.get("input_sha256")
        if case.get("num_statements"):
            extra["statements_in_request"] = int(case["num_statements"])
        if raw.get("after_sql"):
            extra["after_sql"] = raw["after_sql"]
        accepted = raw.get("outcome") in ("accepted_executed", "compiled")
        try:
            imported.receipts.append(
                receipt(
                    Observation(
                        run_id=run.name,
                        audit_sha=audit_sha,
                        raw_bytes=line,
                        case_id=row_id,
                        packet_sha=start["packet_sha256"],
                        sql_sha=sql_sha,
                        setup=[],
                        session=session,
                        method=CAPTURE_EXPLAIN if explain else CAPTURE_EXECUTE,
                        accepted=accepted,
                        errno=errno,
                        sqlstate=sqlstate,
                        message=message,
                    ),
                    **extra,
                )
            )
            count += 1
        except ValueError as error:
            refuse(imported, run.name, row_id, str(error))
    return count


def import_diagnostic_run(imported: Imported, run: Path, events: list[Row], audit_sha: str) -> int:
    """The diagnostic run's raw lines carry their own SQL and session parameters."""
    start = events[0]
    count = 0
    for line in raw_lines(run, events):
        raw = json.loads(line)
        row_id = raw.get("id", "?")
        if raw.get("category") in CONTROLLER_CATEGORIES:
            refuse(imported, run.name, row_id, "controller metadata request, not a syntax case")
            continue
        sql = raw.get("sql")
        session = {**(raw.get("session_parameters") or {}), **(raw.get("safety") or {})}
        if not isinstance(sql, str) or not raw.get("session_parameters"):
            refuse(imported, run.name, row_id, "raw line lacks exact SQL or session parameters")
            continue
        sql_sha = add_input(imported, sql, [])
        errno, sqlstate, message = error_fields(raw)
        try:
            imported.receipts.append(
                receipt(
                    Observation(
                        run_id=run.name,
                        audit_sha=audit_sha,
                        raw_bytes=line,
                        case_id=row_id,
                        packet_sha=start["proposal_sha256"],
                        sql_sha=sql_sha,
                        setup=[],
                        session=session,
                        method=CAPTURE_EXECUTE,
                        accepted=raw.get("outcome") == "rows",
                        errno=errno,
                        sqlstate=sqlstate,
                        message=message,
                    ),
                    query_id=raw.get("query_id"),
                    warehouse_version=raw.get("warehouse_version"),
                    recorded_at=raw.get("started_at"),
                    category=raw.get("category"),
                )
            )
            count += 1
        except ValueError as error:
            refuse(imported, run.name, row_id, str(error))
    return count


def import_canonical_run(
    imported: Imported, run: Path, events: list[Row], audit_sha: str, index: dict[str, Path]
) -> int:
    """Canonical recorder runs: one result document per case, bound by the audit's file hashes."""
    start = events[0]
    binding = start["binding"]
    manifest_path = index.get(binding.get("manifest_sha256", ""))
    if manifest_path is None:
        raise RunRefused(f"manifest {binding.get('manifest_sha256')} not found among the inputs")
    manifest = json.loads(manifest_path.read_text())
    by_hash = {case["case_hash"]: case for case in manifest["cases"]}
    files = completed(events).get("files") or {}
    results = Path(start["results"])
    count = 0
    for case_hash in binding["cases"]:
        result_path = results / f"{case_hash}.json"
        if not result_path.is_file() or digest(result_path.read_bytes()) != files.get(
            result_path.name
        ):
            refuse(
                imported, run.name, case_hash, "result document missing or differs from the audit"
            )
            continue
        pending_path = manifest_path.parent / "pending" / f"{case_hash}.json"
        case = by_hash.get(case_hash)
        if case is None or not pending_path.is_file():
            refuse(imported, run.name, case_hash, "case is not in the bound manifest")
            continue
        pending = json.loads(pending_path.read_text())
        sql = pending.get("source_sql")
        if not isinstance(sql, str) or digest(sql.encode()) != case.get("source_sql_sha256"):
            refuse(imported, run.name, case_hash, "pending SQL does not match the manifest's hash")
            continue
        result = json.loads(result_path.read_bytes())
        pinned = case.get("pinned_parameters") or {}
        session = {**pinned, **(result.get("session_parameters") or {})}
        if not result.get("session_parameters"):
            refuse(imported, run.name, case_hash, "result records no session parameters")
            continue
        setup = list(case.get("setup_sql") or [])
        sql_sha = add_input(imported, sql, setup)
        error = result.get("error") or {}
        try:
            imported.receipts.append(
                receipt(
                    Observation(
                        run_id=run.name,
                        audit_sha=audit_sha,
                        raw_bytes=result_path.read_bytes(),
                        case_id=case.get("reference", case_hash),
                        packet_sha=binding["manifest_sha256"],
                        sql_sha=sql_sha,
                        setup=setup,
                        session=session,
                        method=CAPTURE_CANONICAL,
                        accepted=result.get("outcome") in ("rows", "types"),
                        errno=error.get("errno"),
                        sqlstate=error.get("sqlstate"),
                        message=str(error.get("message") or ""),
                    ),
                    query_id=None,
                    query_id_sha256=result.get("query_id_sha256"),
                    warehouse_version=result.get("warehouse_version"),
                    recorded_at=result.get("recorded_at"),
                    case_hash=case_hash,
                )
            )
            count += 1
        except ValueError as failure:
            refuse(imported, run.name, case_hash, str(failure))
    return count


def refuse(imported: Imported, run_id: str, row_id: str, reason: str) -> None:
    imported.refusals.append({"run_id": run_id, "row_id": row_id, "reason": reason})


def import_runs(runs: Path, input_dirs: list[Path]) -> Imported:
    """Every run directory under ``runs``, imported or refused with a reason."""
    imported = Imported()
    index = index_inputs(input_dirs)
    for run in sorted(path for path in runs.iterdir() if path.is_dir()):
        before = len(imported.refusals)
        try:
            events, audit_sha = read_audit(run)
            start = events[0]
            if (
                start.get("submission") == SCRIPTING_SUBMISSION
                or start.get("method") == EXPLAIN_METHOD
            ):
                count = import_packet_run(imported, run, events, audit_sha, index)
            elif "proposal_sha256" in start:
                count = import_diagnostic_run(imported, run, events, audit_sha)
            elif "manifest_sha256" in (start.get("binding") or {}):
                count = import_canonical_run(imported, run, events, audit_sha, index)
            else:
                raise RunRefused("start event matches no known capture driver")
            status = {"run_id": run.name, "status": "imported", "audit_sha256": audit_sha}
        except RunRefused as error:
            refuse(imported, run.name, "*", str(error))
            count = 0
            status = {"run_id": run.name, "status": "refused", "reason": str(error)}
        status.update(receipts=count, refused_rows=len(imported.refusals) - before)
        imported.runs.append(status)
    return imported


def write_new(path: Path, data: bytes) -> None:
    """Receipts are append-only: never replace an existing file."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as handle:
        handle.write(data)


def run_import(runs: Path, input_dirs: list[Path], receipts_path: Path, report_path: Path) -> Row:
    imported = import_runs(runs, input_dirs)
    lines = b"".join(canonical_json(r) + b"\n" for r in imported.receipts)
    report = {
        "format": RECEIPT_FORMAT,
        "receipts_file_sha256": digest(lines),
        "receipts": len(imported.receipts),
        "refused": len(imported.refusals),
        "runs": imported.runs,
        "refusals": imported.refusals,
        "inputs": sorted(imported.inputs.values(), key=lambda row: row["sql_sha256"]),
    }
    write_new(receipts_path, lines)
    write_new(
        report_path,
        json.dumps(report, indent=1, sort_keys=True, ensure_ascii=False).encode() + b"\n",
    )
    return {"receipts": len(imported.receipts), "refused": len(imported.refusals)}
