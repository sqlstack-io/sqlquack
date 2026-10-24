"""A fake warehouse session and helpers for recorder tests."""

from __future__ import annotations

import hashlib
import json
from collections.abc import Sequence
from decimal import Decimal
from pathlib import Path

import pyarrow as pa

from sqlquack_record.models import (
    ColumnDescription,
    PendingCase,
    RecordedError,
    SessionValue,
    StatementResult,
)
from sqlquack_record.recording.main.record_pending import record_pending
from sqlquack_record.types import WarehouseSession

_COLUMNS: tuple[ColumnDescription, ...] = (
    ColumnDescription("ORDINAL", 0, "FIXED", 1, 0, None, False),
    ColumnDescription("VALUE", 0, "FIXED", 38, 2, None, True),
)


def fake_table(statement: str) -> pa.Table:
    """The rows the fake warehouse returns for a statement."""
    return pa.table(
        {
            "ORDINAL": pa.array([1, 2], type=pa.int8()),
            "VALUE": pa.array([Decimal(len(statement)) / 100, None], type=pa.decimal128(38, 2)),
        }
    )


class FakeSession:
    """Returns scripted results; statements in `failing` raise a recordable SQL error."""

    def __init__(self, failing: frozenset[str], log: list[str]) -> None:
        self._failing: frozenset[str] = failing
        self._log: list[str] = log

    def warehouse_version(self) -> str:
        return "0.0.0-fake"

    def connector(self) -> str:
        return "fake-connector 0"

    def run_batch(self, statements: Sequence[str]) -> list[StatementResult] | None:
        self._log.append("batch")
        if any(statement in self._failing for statement in statements):
            return None
        return [self._rows(statement) for statement in statements]

    def run_one(self, statement: str) -> StatementResult:
        self._log.append("one")
        if statement in self._failing:
            return StatementResult(
                query_id="fake-error-query",
                columns=(),
                table=None,
                error=RecordedError(errno=100051, sqlstate="22012", message="Division by zero"),
            )
        return self._rows(statement)

    def close(self) -> None:
        self._log.append("close")

    def _rows(self, statement: str) -> StatementResult:
        return StatementResult(
            query_id=f"fake-query-{len(statement)}",
            columns=_COLUMNS,
            table=fake_table(statement),
            error=None,
        )


def pending_case(statement: str, parameters: tuple[tuple[str, SessionValue], ...]) -> PendingCase:
    """A pending case whose hash derives from its statement."""
    return PendingCase(
        case_hash=hashlib.sha256(statement.encode("utf-8")).hexdigest(),
        case_reference=f"fake.toml#{len(statement)}",
        source_dialect="snowflake",
        session_parameters=parameters,
        source_sql=statement,
        duckdb_version="v2.0.0",
    )


def record(
    directory: Path,
    statements: Sequence[str],
    failing: frozenset[str],
    parameters: Sequence[tuple[tuple[str, SessionValue], ...]],
    preexisting: Sequence[str],
) -> tuple[int, int, list[str], list[str]]:
    """Record with a fake session; returns counts, the session log and each case's outcome."""
    cases: list[PendingCase] = [
        pending_case(statement, parameters[index % len(parameters)])
        for index, statement in enumerate(statements)
    ]
    results: Path = directory / "results"
    results.mkdir()
    for statement in preexisting:
        (results / f"{pending_case(statement, ()).case_hash}.json").write_text(
            "{}", encoding="utf-8"
        )
    log: list[str] = []

    def open_session(_: tuple[tuple[str, SessionValue], ...]) -> WarehouseSession:
        log.append("open")
        return FakeSession(failing, log)

    summary = record_pending(cases, results, open_session, 10, lambda: "2026-01-01T00:00:00Z")
    outcomes: list[str] = [_outcome(results, case) for case in cases]
    return summary.recorded, summary.errors, log, outcomes


def _outcome(results: Path, case: PendingCase) -> str:
    metadata: dict[str, object] = json.loads(
        (results / f"{case.case_hash}.json").read_text(encoding="utf-8")
    )
    if not metadata:
        return "preexisting"
    arrow_path: Path = results / f"{case.case_hash}.arrow"
    if metadata["outcome"] == "error":
        return "error" if not arrow_path.exists() else "error-with-arrow"
    arrow_bytes: bytes = arrow_path.read_bytes()
    table: pa.Table = pa.ipc.open_file(pa.BufferReader(arrow_bytes)).read_all()
    intact: bool = (
        table.equals(fake_table(case.source_sql))
        and metadata["arrow_sha256"] == hashlib.sha256(arrow_bytes).hexdigest()
        and metadata["row_count"] == table.num_rows
        and metadata["case_hash"] == case.case_hash
        and metadata["warehouse_version"] == "0.0.0-fake"
    )
    return "rows" if intact else "rows-altered"
