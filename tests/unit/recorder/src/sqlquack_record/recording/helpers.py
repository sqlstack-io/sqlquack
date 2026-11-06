"""A fake warehouse session and helpers for recorder tests."""

from __future__ import annotations

import hashlib
import json
import re
from collections.abc import Sequence
from decimal import Decimal
from pathlib import Path
from typing import cast

import pyarrow as pa

from sqlquack_record.recording.exceptions import RecorderError, StatementNotRecordableError
from sqlquack_record.recording.main.record_pending import record_pending
from sqlquack_record.recording.main.record_session_defaults import record_session_defaults
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
_SCRATCH_SUFFIX: re.Pattern[str] = re.compile(r"__sqlquack_[0-9a-f]+_[0-9a-f]+")
_PINNED: SessionParameters = (("DATE_OUTPUT_FORMAT", "YYYY-MM-DD"),)
_FUNCTIONS: tuple[str, ...] = (
    "CREATE FUNCTION order_total(qty NUMBER) RETURNS NUMBER AS $$ qty $$",
)
_FAILURE: StatementResult = StatementResult(
    query_id="fake-error-query",
    columns=(),
    table=None,
    failure=WarehouseFailure(errno=100051, sqlstate="22012"),
)


_COMPILE_FAILURE: WarehouseFailure = WarehouseFailure(
    errno=1003, sqlstate="42000", message="syntax error line 1 at position 10 unexpected 'order'"
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
        self,
        *,
        responses: dict[str, StatementResult],
        batch_fails: bool,
        log: list[str],
        parameters: SessionParameters = (),
    ) -> None:
        self._responses: dict[str, StatementResult] = responses
        self._parameters: SessionParameters = parameters
        self.broken: frozenset[str] = frozenset()
        self.uncompilable: frozenset[str] = frozenset()
        self._batch_fails: bool = batch_fails
        self._log: list[str] = log

    def warehouse_version(self) -> str:
        return "0.0.0-fake"

    def connector(self) -> str:
        return "fake-connector 0"

    def run_batch(self, statements: Sequence[str]) -> list[StatementResult] | None:
        self._log.append("batch")
        for _ in self.broken.intersection(statements):
            raise StatementNotRecordableError("Snowflake error 2139 is not a case outcome")
        answers: dict[bool, list[StatementResult] | None] = {
            False: [self._responses[statement] for statement in statements],
            True: None,
        }
        return answers[self._batch_fails]

    def run_one(self, *, statement: str, expected_error: bool = False) -> StatementResult:
        self._log.append("one")
        for _ in self.broken.intersection({statement}):
            raise StatementNotRecordableError("Snowflake error 2139 is not a case outcome")
        return self._responses[_SCRATCH_SUFFIX.sub("", statement)]

    def compile_failure(self, statement: str) -> WarehouseFailure | None:
        failures: dict[bool, WarehouseFailure | None] = {True: _COMPILE_FAILURE, False: None}
        return failures[statement in self.uncompilable]

    def execute(self, statement: str) -> None:
        self._log.append(" ".join(statement.split()[:2]).lower())

    def current_schema(self) -> str | None:
        return "ORDERS"

    def session_parameters(self) -> dict[str, str]:
        return {"QUERY_TAG": "fake"} | {name: str(value) for name, value in self._parameters}

    def close(self) -> None:
        self._log.append("close")


def _pending_case(
    *,
    statement: str,
    parameters: SessionParameters,
    values: bool = True,
    with_functions: bool = False,
    compile_error: bool = False,
) -> PendingCase:
    return PendingCase(
        case_hash=hashlib.sha256(statement.encode("utf-8")).hexdigest(),
        case_reference=f"fake.toml#{len(statement)}",
        source_dialect="snowflake",
        session_parameters=parameters,
        source_sql=statement,
        duckdb_version="v2.0.0",
        values=values,
        setup_sql=_FUNCTIONS * with_functions,
        pinned_parameters=_PINNED,
        compile_error=compile_error,
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
        json.dumps(metadata.get("row_count")),
    )


def record(
    *,
    directory: Path,
    statements: Sequence[str],
    failing: frozenset[str],
    batch_fails: bool,
    parameters: Sequence[SessionParameters],
    preexisting: Sequence[str],
    without_values: frozenset[str] = frozenset(),
    with_functions: frozenset[str] = frozenset(),
) -> tuple[int, int, list[str], list[RecordedFile]]:
    """Record with a fake session; returns counts, the session log and what each case stored."""
    cases: list[PendingCase] = [
        _pending_case(
            statement=statement,
            parameters=parameters[index % len(parameters)],
            values=statement not in without_values,
            with_functions=statement in with_functions,
        )
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

    def open_session(opened: SessionParameters) -> WarehouseSession:
        log.append("open")
        return FakeSession(responses=responses, batch_fails=batch_fails, log=log, parameters=opened)

    summary: RecordSummary = record_pending(
        cases=cases,
        results=results,
        open_session=open_session,
        batch_size=10,
        clock=lambda: "2026-01-01T00:00:00Z",
    )
    files: list[RecordedFile] = [_recorded_file(results=results, case=case) for case in cases]
    return summary.recorded, summary.failures, log, files


def broken_case_message(*, directory: Path, statements: Sequence[str], broken: str) -> str:
    """The recorder's error when one statement of a batch does not compile on the warehouse."""
    cases: list[PendingCase] = [
        _pending_case(statement=statement, parameters=()) for statement in statements
    ]
    responses: dict[str, StatementResult] = {
        statement: _rows(statement) for statement in statements
    }

    def open_session(_: SessionParameters) -> WarehouseSession:
        session: FakeSession = FakeSession(responses=responses, batch_fails=False, log=[])
        session.broken = frozenset({broken})
        return session

    try:
        record_pending(
            cases=cases,
            results=directory,
            open_session=open_session,
            batch_size=10,
            clock=lambda: "2026-01-01T00:00:00Z",
        )
    except RecorderError as error:
        return str(error)
    return ""


def precheck(
    *,
    directory: Path,
    statements: Sequence[str],
    uncompilable: frozenset[str],
    declared: frozenset[str],
    with_functions: frozenset[str],
) -> tuple[str, list[str], int]:
    """Record cases some of which do not compile; the error, the session log and files written."""
    cases: list[PendingCase] = [
        _pending_case(
            statement=statement,
            parameters=(),
            with_functions=statement in with_functions,
            compile_error=statement in declared,
        )
        for statement in statements
    ]
    responses: dict[str, StatementResult] = {
        statement: _rows(statement) for statement in statements
    }
    log: list[str] = []

    def open_session(_: SessionParameters) -> WarehouseSession:
        log.append("open")
        session: FakeSession = FakeSession(responses=responses, batch_fails=False, log=log)
        session.uncompilable = uncompilable
        return session

    results: Path = directory / "results"
    message: str = ""
    try:
        record_pending(
            cases=cases,
            results=results,
            open_session=open_session,
            batch_size=10,
            clock=lambda: "2026-01-01T00:00:00Z",
        )
    except RecorderError as error:
        message = str(error)
    return message, log, len(list(results.glob("*.json")))


def recorded_parameters(
    *, directory: Path, parameters: SessionParameters
) -> tuple[dict[str, object], bool]:
    """Record one case; its stored effective parameters, and whether the snapshot hash is set."""
    statement: str = "SELECT 1"
    case: PendingCase = _pending_case(statement=statement, parameters=parameters)
    results: Path = directory / "results"
    results.mkdir()

    def open_session(opened: SessionParameters) -> WarehouseSession:
        return FakeSession(
            responses={statement: _rows(statement)}, batch_fails=False, log=[], parameters=opened
        )

    record_pending(
        cases=[case],
        results=results,
        open_session=open_session,
        batch_size=10,
        clock=lambda: "2026-01-01T00:00:00Z",
    )
    metadata: dict[str, object] = json.loads(
        (results / f"{case.case_hash}.json").read_text(encoding="utf-8")
    )
    return (
        cast(dict[str, object], metadata["effective_parameters"]),
        len(str(metadata["parameters_sha256"])) == 64,
    )


def session_defaults(*, directory: Path, names: frozenset[str]) -> dict[str, object]:
    """The defaults file written for `names` from a fake session reporting TIMEZONE."""
    out: Path = directory / "session_defaults.json"

    def open_session(opened: SessionParameters) -> WarehouseSession:
        return FakeSession(
            responses={}, batch_fails=False, log=[], parameters=(("TIMEZONE", "UTC"),)
        )

    record_session_defaults(
        out=out, names=names, open_session=open_session, clock=lambda: "2026-01-01T00:00:00Z"
    )
    return json.loads(out.read_text(encoding="utf-8"))
