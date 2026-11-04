"""Diagnostic runs, whose raw lines carry their own SQL and session parameters."""

from __future__ import annotations

import json

from scripts.syntax_receipts._helpers.audit import connector_query_id, error_fields, raw_lines
from scripts.syntax_receipts._helpers.receipts import input_row, receipt, refusal
from scripts.syntax_receipts.constants import (
    CAPTURE_EXECUTE,
    CONTROLLER_CATEGORIES,
    DIAGNOSTIC_ACCEPTED_OUTCOME,
    UNKNOWN_ROW,
)
from scripts.syntax_receipts.exceptions import RowRefused
from scripts.syntax_receipts.models import CaptureRun, Imported, Observation
from scripts.syntax_receipts.types import Row


def import_diagnostic_run(*, run: CaptureRun) -> Imported:
    """Receipts for the diagnostic syntax requests; controller metadata requests are refused."""
    start: Row = run.events[0]
    found: Imported = Imported()
    for line in raw_lines(run=run):
        raw: Row = json.loads(line)
        row_id: str = raw.get("id", UNKNOWN_ROW)
        try:
            sql, session = _exact_sql(raw=raw)
        except RowRefused as refused:
            found.refusals.append(refusal(run_id=run.path.name, row_id=row_id, reason=str(refused)))
            continue
        exact: Row = input_row(sql=sql, setup=[])
        found.inputs.setdefault(exact["sql_sha256"], exact)
        errno, sqlstate, message = error_fields(raw=raw)
        observation: Observation = Observation(
            run_id=run.path.name,
            audit_sha=run.audit_sha,
            raw_bytes=line,
            case_id=row_id,
            packet_sha=start["proposal_sha256"],
            sql_sha=exact["sql_sha256"],
            setup=[],
            session=session,
            method=CAPTURE_EXECUTE,
            accepted=raw.get("outcome") == DIAGNOSTIC_ACCEPTED_OUTCOME,
            errno=errno,
            sqlstate=sqlstate,
            message=message,
        )
        extra: Row = {
            "query_id": raw.get("query_id") or connector_query_id(raw=raw),
            "warehouse_version": raw.get("warehouse_version"),
            "recorded_at": raw.get("started_at"),
            "category": raw.get("category"),
        }
        try:
            found.receipts.append(receipt(observation=observation, extra=extra))
        except RowRefused as error:
            found.refusals.append(refusal(run_id=run.path.name, row_id=row_id, reason=str(error)))
    return found


def _exact_sql(*, raw: Row) -> tuple[str, Row]:
    if raw.get("category") in CONTROLLER_CATEGORIES:
        raise RowRefused("controller metadata request, not a syntax case")
    sql: object = raw.get("sql")
    session: Row = {**(raw.get("session_parameters") or {}), **(raw.get("safety") or {})}
    if not isinstance(sql, str) or not raw.get("session_parameters"):
        raise RowRefused("raw line lacks exact SQL or session parameters")
    return sql, session
