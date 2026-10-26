"""Writing one recorded result: the Arrow IPC file, then its metadata."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

import pyarrow as pa

from sqlquack_record.recording.constants import (
    ERROR_OUTCOME,
    EVIDENCE_FORMAT,
    RECORDER_NAME,
    RECORDER_VERSION,
    ROWS_OUTCOME,
    TYPES_OUTCOME,
)
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.models import PendingCase, RecordingContext, StatementResult


def write_result(
    *, directory: Path, case: PendingCase, result: StatementResult, context: RecordingContext
) -> None:
    """Store `result` for `case` without converting a single value; a typing case keeps none."""
    arrow_sha256: str | None = None
    row_count: int | None = None
    outcome: str = ERROR_OUTCOME
    if result.table is not None and not case.values:
        row_count = result.table.num_rows
        outcome = TYPES_OUTCOME
    elif result.table is not None:
        arrow_bytes: bytes = _ipc_file(result.table)
        _create_exclusively(path=directory / f"{case.case_hash}.arrow", content=arrow_bytes)
        arrow_sha256 = hashlib.sha256(arrow_bytes).hexdigest()
        row_count = result.table.num_rows
        outcome = ROWS_OUTCOME
    metadata: dict[str, object] = {
        "format": EVIDENCE_FORMAT,
        "case_hash": case.case_hash,
        "outcome": outcome,
        "recorder": f"{RECORDER_NAME} {RECORDER_VERSION}",
        "connector": context.connector,
        "query_id_sha256": None
        if result.query_id is None
        else hashlib.sha256(result.query_id.encode("utf-8")).hexdigest(),
        "warehouse_version": context.warehouse_version,
        "duckdb_version": case.duckdb_version,
        "recorded_at": context.recorded_at,
        "session_parameters": dict(case.session_parameters),
        "effective_parameters": effective_parameters(case=case, context=context),
        "parameters_sha256": parameters_sha256(dict(context.session_parameters)),
        "columns": [
            {
                "name": column.name,
                "type_code": column.type_code,
                "type_name": column.type_name,
                "precision": column.precision,
                "scale": column.scale,
                "internal_size": column.internal_size,
                "nullable": column.nullable,
            }
            for column in result.columns
        ],
        "row_count": row_count,
        "arrow_sha256": arrow_sha256,
        "error": None
        if result.failure is None
        else {
            "errno": result.failure.errno,
            "sqlstate": result.failure.sqlstate,
            "message": result.failure.message,
        },
    }
    _create_exclusively(
        path=directory / f"{case.case_hash}.json",
        content=(json.dumps(metadata, indent=2, sort_keys=True) + "\n").encode("utf-8"),
    )


def effective_parameters(*, case: PendingCase, context: RecordingContext) -> dict[str, str | None]:
    """Each set and pinned parameter's reported value; None for one the session did not report."""
    reported: dict[str, str] = dict(context.session_parameters)
    return {name: reported.get(name) for name, _ in case.recording_parameters()}


def parameters_sha256(parameters: dict[str, str]) -> str:
    """SHA-256 of every parameter as sorted `KEY=value` lines."""
    lines: str = "".join(f"{name}={value}\n" for name, value in sorted(parameters.items()))
    return hashlib.sha256(lines.encode("utf-8")).hexdigest()


def _ipc_file(table: pa.Table) -> bytes:
    sink: pa.BufferOutputStream = pa.BufferOutputStream()
    with pa.ipc.new_file(sink, table.schema) as writer:
        writer.write_table(table)
    return sink.getvalue().to_pybytes()


def _create_exclusively(*, path: Path, content: bytes) -> None:
    try:
        descriptor: int = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    except FileExistsError as error:
        raise RecorderError(f"{path.name} already exists; results are append-only") from error
    with os.fdopen(descriptor, "wb") as handle:
        handle.write(content)
