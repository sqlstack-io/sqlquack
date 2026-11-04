"""Runs in a run-owned database: setup is the run setup, the case schema and its packet setup."""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path

from scripts.syntax_receipts._helpers.audit import (
    connector_query_id,
    error_fields,
    packet_rows,
    raw_lines,
)
from scripts.syntax_receipts._helpers.hashing import canonical_json, digest
from scripts.syntax_receipts._helpers.receipts import input_row, receipt, refusal
from scripts.syntax_receipts.constants import (
    CAPTURE_DDL,
    CAPTURE_EXPLAIN,
    CASE_SCHEMA,
    EXPLAIN_METHOD,
    MEASURE_KEY,
    PACKET_ACCEPTED_OUTCOMES,
    QUERY_HISTORY_FAILED,
    QUERY_HISTORY_SOURCE,
    QUERY_HISTORY_SUCCESS,
    RUN_DATABASE,
    RUN_NONCE,
    SETUP_FAILED_OUTCOME,
    SHARED_SCHEMA_NOTE,
    UNKNOWN_ROW,
    UNRECORDED_KEY,
)
from scripts.syntax_receipts.exceptions import RowRefused
from scripts.syntax_receipts.models import CaptureRun, Imported, Observation
from scripts.syntax_receipts.types import Row


def import_ddl_run(*, run: CaptureRun, index: dict[str, Path]) -> Imported:
    """Receipts for the raw lines whose SQL hash matches the bound packet's exact SQL."""
    start: Row = run.events[0]
    session: Row = {**start["parameters"], **start.get("safety", {})}
    explain: bool = start.get("method") == EXPLAIN_METHOD
    packet: dict[str, Row] = packet_rows(index=index, sha=start.get("packet_sha256"))
    found: Imported = Imported()
    records, refused = _records(run=run, packet=packet)
    found.refusals.extend(refused)
    for position, line, raw in records:
        row_id: str = raw.get("id", UNKNOWN_ROW)
        case: Row | None = packet.get(row_id)
        if case is None or digest(case["sql"].encode()) != raw.get("sql_sha256"):
            found.refusals.append(
                refusal(
                    run_id=run.path.name,
                    row_id=row_id,
                    reason="raw SQL hash does not match the packet's exact SQL",
                )
            )
            continue
        if raw.get("outcome") == SETUP_FAILED_OUTCOME:
            found.refusals.append(
                refusal(
                    run_id=run.path.name,
                    row_id=row_id,
                    reason="fixture setup failed; the statement was not submitted",
                )
            )
            continue
        setup: list[str] = _setup(start=start, raw=raw, position=position) + case.get("setup", [])
        exact: Row = input_row(sql=case["sql"], setup=setup)
        found.inputs.setdefault(exact["sql_sha256"] + exact["setup_sha256"], exact)
        errno, sqlstate, message = error_fields(raw=raw)
        results: list[Row] = raw.get("statement_results") or []
        observation: Observation = Observation(
            run_id=run.path.name,
            audit_sha=run.audit_sha,
            raw_bytes=line,
            case_id=row_id,
            packet_sha=start["packet_sha256"],
            sql_sha=raw["sql_sha256"],
            setup=setup,
            session=session,
            method=CAPTURE_EXPLAIN if explain else CAPTURE_DDL,
            accepted=raw.get("outcome") in PACKET_ACCEPTED_OUTCOMES,
            errno=errno,
            sqlstate=sqlstate,
            message=message,
        )
        extra: Row = {
            "query_id": (results[0]["query_id"] if results else raw.get("query_id"))
            or connector_query_id(raw=raw),
            "warehouse_version": raw.get("warehouse_version"),
            "recorded_at": raw.get("started_at"),
            "statements_in_request": raw.get("statements"),
            "completed_statements": raw.get("completed_statements"),
        }
        if start.get(MEASURE_KEY):
            extra[MEASURE_KEY] = start[MEASURE_KEY]
        if explain:
            extra.update(
                statement_index=raw.get("statement_index"),
                statements_in_input=case.get("statements_in_input"),
                input_sha256=case.get("input_sha256"),
            )
        try:
            row: Row = receipt(observation=observation, extra=extra)
        except RowRefused as error:
            found.refusals.append(refusal(run_id=run.path.name, row_id=row_id, reason=str(error)))
            continue
        if raw.get("source") == QUERY_HISTORY_SOURCE:
            row["phase_basis"] += (
                f"; from query history {row['query_id']}, the run stopped before its raw record"
            )
        found.receipts.append(row)
    return found


def _records(
    *, run: CaptureRun, packet: dict[str, Row]
) -> tuple[list[tuple[int, bytes, Row]], list[Row]]:
    """The run's raw records and any reconciled submission, with positions, and refused lines."""
    lines: list[bytes] = raw_lines(run=run)
    records: list[tuple[int, bytes, Row]] = []
    refused: list[Row] = []
    for position, line in enumerate(lines):
        try:
            parsed: object = json.loads(line)
        except ValueError:
            parsed = None
        if not isinstance(parsed, dict):
            refused.append(
                refusal(run_id=run.path.name, row_id=UNKNOWN_ROW, reason="raw record is not JSON")
            )
            continue
        records.append((position, line, parsed))
    reconciliation: Row = run.reconciliation or {}
    if UNRECORDED_KEY in reconciliation:
        unrecorded: object = reconciliation[UNRECORDED_KEY]
        try:
            line, raw = _unrecorded(entry=unrecorded, packet=packet, records=records)
            records.append((len(lines), line, raw))
        except RowRefused as error:
            case_id: object = unrecorded.get("case_id") if isinstance(unrecorded, dict) else None
            refused.append(
                refusal(
                    run_id=run.path.name,
                    row_id=case_id if isinstance(case_id, str) else UNKNOWN_ROW,
                    reason=str(error),
                )
            )
    return records, refused


def _unrecorded(
    *, entry: object, packet: dict[str, Row], records: list[tuple[int, bytes, Row]]
) -> tuple[bytes, Row]:
    """A reconciled submission without a raw record, as a raw record from its query-history row."""
    if not isinstance(entry, dict):
        raise RowRefused("unrecorded submission is not an object")
    case_id: object = entry.get("case_id")
    case: Row | None = packet.get(case_id) if isinstance(case_id, str) else None
    if entry.get("source") != QUERY_HISTORY_SOURCE:
        raise RowRefused("unrecorded submission is not from query history")
    if not _text(entry.get("query_id")):
        raise RowRefused("unrecorded submission carries no query-history query id")
    if case is None or not (
        entry.get("sql_sha256") == entry.get("query_text_sha256") == digest(case["sql"].encode())
    ):
        raise RowRefused("unrecorded submission SQL hash does not match the packet's exact SQL")
    if any(raw.get("id") == case_id for _, _, raw in records):
        raise RowRefused("unrecorded submission has a raw record")
    if entry.get("sqlstate") is not None:
        raise RowRefused("unrecorded submission carries a SQLSTATE, which query history lacks")
    if not _text(entry.get("release_version")):
        raise RowRefused("unrecorded submission carries no query-history release_version")
    status: object = entry.get("execution_status")
    errno: object = entry.get("errno")
    succeeded: bool = status == QUERY_HISTORY_SUCCESS and errno is None
    failed: bool = (
        status == QUERY_HISTORY_FAILED and type(errno) is int and _text(entry.get("error_message"))
    )
    if not (succeeded or failed):
        raise RowRefused(
            f"unrecorded submission status {status!r} with errno {errno!r} is not a finished query"
        )
    start_time: object = entry.get("start_time")
    try:
        started_at: str = (
            datetime.fromisoformat(start_time).isoformat() if isinstance(start_time, str) else ""
        )
    except ValueError:
        started_at = ""
    if not started_at:
        raise RowRefused("unrecorded submission has no readable start time")
    raw: Row = {
        "id": case_id,
        "sql_sha256": entry["sql_sha256"],
        "outcome": PACKET_ACCEPTED_OUTCOMES[0] if succeeded else "error",
        "connector_fields": {
            "errno": errno,
            "sqlstate": None,
            "raw_msg": entry.get("error_message") or "",
        },
        "query_id": entry["query_id"],
        "case_schema": entry.get("case_schema"),
        "started_at": started_at,
        "warehouse_version": entry["release_version"],
        "source": QUERY_HISTORY_SOURCE,
    }
    return canonical_json(entry), raw


def _text(value: object) -> bool:
    return isinstance(value, str) and bool(value)


def _setup(*, start: Row, raw: Row, position: int) -> list[str]:
    database: str = start["run_database"]
    nonce: str = database.rsplit("_", 1)[-1]
    setup: list[str] = [
        statement.replace(database, RUN_DATABASE).replace(nonce, RUN_NONCE)
        for statement in start["setup"]
    ]
    if raw.get("case_schema"):
        setup += [
            f"CREATE SCHEMA {RUN_DATABASE}.{CASE_SCHEMA}",
            f"USE SCHEMA {RUN_DATABASE}.{CASE_SCHEMA}",
        ]
    else:
        setup.append(f"{SHARED_SCHEMA_NOTE} {position}")
    return setup
