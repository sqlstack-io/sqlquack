"""Write one recorded result: the Arrow IPC file, then its metadata."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pyarrow as pa

from sqlquack_record.constants import EVIDENCE_FORMAT, RECORDER_NAME, RECORDER_VERSION
from sqlquack_record.models import PendingCase, RecordingContext, StatementResult
from sqlquack_record.results._helpers.files import create_exclusively


def write_result(
    directory: Path, case: PendingCase, result: StatementResult, context: RecordingContext
) -> None:
    """Store `result` for `case` without converting a single value."""
    arrow_sha256: str | None = None
    row_count: int | None = None
    if result.table is not None:
        arrow_bytes: bytes = _ipc_file(result.table)
        create_exclusively(directory / f"{case.case_hash}.arrow", arrow_bytes)
        arrow_sha256 = hashlib.sha256(arrow_bytes).hexdigest()
        row_count = result.table.num_rows
    metadata: dict[str, object] = {
        "format": EVIDENCE_FORMAT,
        "case_hash": case.case_hash,
        "outcome": "rows" if result.table is not None else "error",
        "recorder": f"{RECORDER_NAME} {RECORDER_VERSION}",
        "connector": context.connector,
        "query_id": result.query_id,
        "warehouse_version": context.warehouse_version,
        "duckdb_version": case.duckdb_version,
        "recorded_at": context.recorded_at,
        "session_parameters": dict(case.session_parameters),
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
        if result.error is None
        else {
            "errno": result.error.errno,
            "sqlstate": result.error.sqlstate,
            "message": result.error.message,
        },
    }
    create_exclusively(
        directory / f"{case.case_hash}.json",
        (json.dumps(metadata, indent=2, sort_keys=True) + "\n").encode("utf-8"),
    )


def _ipc_file(table: pa.Table) -> bytes:
    sink: pa.BufferOutputStream = pa.BufferOutputStream()
    with pa.ipc.new_file(sink, table.schema) as writer:
        writer.write_table(table)
    return sink.getvalue().to_pybytes()
