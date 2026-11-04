"""Runs in a run-owned database: setup is the run setup, the case schema and its packet setup."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.syntax_receipts._helpers.audit import (
    connector_query_id,
    error_fields,
    packet_rows,
    raw_lines,
)
from scripts.syntax_receipts._helpers.hashing import digest
from scripts.syntax_receipts._helpers.receipts import input_row, receipt, refusal
from scripts.syntax_receipts.constants import (
    CAPTURE_DDL,
    CAPTURE_EXPLAIN,
    CASE_SCHEMA,
    EXPLAIN_METHOD,
    MEASURE_KEY,
    PACKET_ACCEPTED_OUTCOMES,
    RUN_DATABASE,
    RUN_NONCE,
    SETUP_FAILED_OUTCOME,
    SHARED_SCHEMA_NOTE,
    UNKNOWN_ROW,
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
    for position, line in enumerate(raw_lines(run=run)):
        raw: Row = json.loads(line)
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
            found.receipts.append(receipt(observation=observation, extra=extra))
        except RowRefused as error:
            found.refusals.append(refusal(run_id=run.path.name, row_id=row_id, reason=str(error)))
    return found


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
