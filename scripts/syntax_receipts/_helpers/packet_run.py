"""Anonymous-block, execute and EXPLAIN runs: a raw line per packet row or statement."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.syntax_receipts._helpers.audit import (
    client_failure,
    connector_query_id,
    error_fields,
    packet_rows,
    raw_lines,
)
from scripts.syntax_receipts._helpers.hashing import digest
from scripts.syntax_receipts._helpers.receipts import input_row, receipt, refusal
from scripts.syntax_receipts.constants import (
    CAPTURE_EXECUTE,
    CAPTURE_EXPLAIN,
    EXPLAIN_METHOD,
    PACKET_ACCEPTED_OUTCOMES,
    SQL_KEY,
    UNKNOWN_ROW,
)
from scripts.syntax_receipts.exceptions import RowRefused, RunRefused
from scripts.syntax_receipts.models import CaptureRun, Imported, Observation
from scripts.syntax_receipts.types import Row


def import_packet_run(*, run: CaptureRun, index: dict[str, Path]) -> Imported:
    """Receipts for the raw lines whose SQL hash matches the bound packet's exact SQL."""
    start: Row = run.events[0]
    explain: bool = start.get("method") == EXPLAIN_METHOD
    session: Row = {**start.get("parameters", {}), **start.get("safety", {})}
    if not start.get("parameters"):
        raise RunRefused("start event records no session parameters")
    packet: dict[str, Row] = packet_rows(index=index, sha=start.get("packet_sha256"))
    found: Imported = Imported()
    for line in raw_lines(run=run):
        raw: Row = json.loads(line)
        row_id: str = raw.get("id", UNKNOWN_ROW)
        try:
            case: Row = _bound_case(packet=packet, raw=raw)
        except RowRefused as refused:
            found.refusals.append(refusal(run_id=run.path.name, row_id=row_id, reason=str(refused)))
            continue
        failure: str | None = client_failure(raw=raw)
        if failure is not None:
            found.refusals.append(refusal(run_id=run.path.name, row_id=row_id, reason=failure))
            continue
        sql: Row = input_row(sql=case["sql"], setup=[])
        found.inputs.setdefault(sql["sql_sha256"] + sql["setup_sha256"], sql)
        errno, sqlstate, message = error_fields(raw=raw)
        observation: Observation = Observation(
            run_id=run.path.name,
            audit_sha=run.audit_sha,
            raw_bytes=line,
            case_id=row_id,
            packet_sha=start["packet_sha256"],
            sql_sha=raw["sql_sha256"],
            setup=[],
            session=session,
            method=CAPTURE_EXPLAIN if explain else CAPTURE_EXECUTE,
            accepted=raw.get("outcome") in PACKET_ACCEPTED_OUTCOMES,
            errno=errno,
            sqlstate=sqlstate,
            message=message,
        )
        try:
            found.receipts.append(
                receipt(observation=observation, extra=_extra(raw=raw, case=case, explain=explain))
            )
        except RowRefused as error:
            found.refusals.append(refusal(run_id=run.path.name, row_id=row_id, reason=str(error)))
    return found


def _bound_case(*, packet: dict[str, Row], raw: Row) -> Row:
    case: Row | None = packet.get(raw.get("id", UNKNOWN_ROW))
    if case is None or SQL_KEY not in case:
        raise RowRefused("row id is not in the bound packet")
    sql_sha: str | None = raw.get("sql_sha256")
    if not sql_sha or sql_sha != case.get("sql_sha256") or digest(case["sql"].encode()) != sql_sha:
        raise RowRefused("raw SQL hash does not match the packet's exact SQL")
    return case


def _extra(*, raw: Row, case: Row, explain: bool) -> Row:
    extra: Row = {
        "query_id": raw.get("query_id") or connector_query_id(raw=raw),
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
    return extra
