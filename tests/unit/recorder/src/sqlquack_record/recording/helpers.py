"""A fake warehouse session and helpers for recorder tests."""

from __future__ import annotations

import hashlib
import json
from collections.abc import Sequence
from decimal import Decimal
from pathlib import Path

import pyarrow as pa

from sqlquack_record.recording.main.record_pending import record_pending
from sqlquack_record.recording.models import (
    ColumnDescription,
    PendingCase,
    RecordSummary,
    StatementResult,
    WarehouseFailure,
)
from sqlquack_record.recording.types import SessionParameters, WarehouseSession
from tests.unit.recorder.src.sqlquack_record.recording._test_types import RecordedFile

_COLUMNS: tuple[ColumnDescription, ...] = (
    ColumnDescription("ORDINAL", 0, "FIXED", 1, 0, None, False),
    ColumnDescription("VALUE", 0, "FIXED", 38, 2, None, True),
)
_FAILURE: StatementResult = StatementResult(
    query_id="fake-error-query",
    columns=(),
    table=None,
    failure=WarehouseFailure(errno=100051, sqlstate="22012"),
)


def fake_table(statement: str) -> pa.Table:
    """The rows the fake warehouse returns for a statement."""
    return pa.table(
        {
            "ORDINAL": pa.array([1, 2], type=pa.int8()),
            "VALUE": pa.array([Decimal(len(statement)) / 100, None], type=pa.decimal128(38, 2)),
        }
    )


def _rows(statement: str) -> StatementResult:
    return StatementResult(
        query_id=f"fake-query-{len(statement)}",
        columns=_COLUMNS,
        table=fake_table(statement),
        failure=None,
    )


class FakeSession:
    """Answers from scripted responses; a failing batch returns `None` like a SQL error does."""

    def __init__(
        self, *, responses: dict[str, StatementResult], batch_fails: bool, log: list[str]
    ) -> None:
        self._responses: dict[str, StatementResult] = responses
        self._batch_fails: bool = batch_fails
        self._log: list[str] = log

    def warehouse_version(self) -> str:
        return "0.0.0-fake"

    def connector(self) -> str:
        return "fake-connector 0"

    def run_batch(self, statements: Sequence[str]) -> list[StatementResult] | None:
        self._log.append("batch")
        answers: dict[bool, list[StatementResult] | None] = {
            False: [self._responses[statement] for statement in statements],
            True: None,
        }
        return answers[self._batch_fails]

    def run_one(self, statement: str) -> StatementResult:
        self._log.append("one")
        return self._responses[statement]

    def close(self) -> None:
        self._log.append("close")


def _pending_case(*, statement: str, parameters: SessionParameters) -> PendingCase:
    return PendingCase(
        case_hash=hashlib.sha256(statement.encode("utf-8")).hexdigest(),
        case_reference=f"fake.toml#{len(statement)}",
        source_dialect="snowflake",
        session_parameters=parameters,
        source_sql=statement,
        duckdb_version="v2.0.0",
    )


def _recorded_file(*, results: Path, case: PendingCase) -> RecordedFile:
    text: str = (results / f"{case.case_hash}.json").read_text(encoding="utf-8")
    metadata: dict[str, object] = json.loads(text)
    arrow_files: list[bytes] = [
        path.read_bytes() for path in results.glob(f"{case.case_hash}.arrow")
    ]
    claimed: list[object] = [metadata.get("arrow_sha256")]
    return (
        str(metadata.get("outcome", "preexisting")),
        "-query" not in text
        and "query-" not in text
        and len(str(metadata.get("query_id_sha256", ""))) == 64,
        [hashlib.sha256(data).hexdigest() for data in arrow_files] == list(filter(None, claimed)),
        all(
            pa.ipc.open_file(pa.BufferReader(data)).read_all().equals(fake_table(case.source_sql))
            for data in arrow_files
        ),
    )


def record(
    *,
    directory: Path,
    statements: Sequence[str],
    failing: frozenset[str],
    batch_fails: bool,
    parameters: Sequence[SessionParameters],
    preexisting: Sequence[str],
) -> tuple[int, int, list[str], list[RecordedFile]]:
    """Record with a fake session; returns counts, the session log and what each case stored."""
    cases: list[PendingCase] = [
        _pending_case(statement=statement, parameters=parameters[index % len(parameters)])
        for index, statement in enumerate(statements)
    ]
    results: Path = directory / "results"
    results.mkdir()
    for statement in preexisting:
        stored: PendingCase = _pending_case(statement=statement, parameters=())
        (results / f"{stored.case_hash}.json").write_text("{}", encoding="utf-8")
    responses: dict[str, StatementResult] = {
        **{statement: _rows(statement) for statement in statements},
        **{statement: _FAILURE for statement in failing},
    }
    log: list[str] = []

    def open_session(_: SessionParameters) -> WarehouseSession:
        log.append("open")
        return FakeSession(responses=responses, batch_fails=batch_fails, log=log)

    summary: RecordSummary = record_pending(
        cases=cases,
        results=results,
        open_session=open_session,
        batch_size=10,
        clock=lambda: "2026-01-01T00:00:00Z",
    )
    files: list[RecordedFile] = [_recorded_file(results=results, case=case) for case in cases]
    return summary.recorded, summary.failures, log, files
