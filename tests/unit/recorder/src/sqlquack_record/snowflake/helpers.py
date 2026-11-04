"""Connection arguments against a fake configuration, and a fake connection for sessions."""

from __future__ import annotations

import hashlib
import json
import re
from collections import defaultdict
from collections.abc import Callable, Sequence
from decimal import Decimal
from functools import partial
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pyarrow as pa
import pytest
from snowflake.connector.errors import NotSupportedError, ProgrammingError

from sqlquack_record.recording.exceptions import RecorderError, UnrecordableResultError
from sqlquack_record.recording.main.record_pending import record_pending
from sqlquack_record.recording.models import PendingCase, RecordSummary, StatementResult
from sqlquack_record.recording.types import (
    SessionParameters,
    SubMicrosecondCheck,
    WarehouseSession,
)
from sqlquack_record.snowflake._helpers import connections
from sqlquack_record.snowflake._helpers.connections import connect_arguments
from sqlquack_record.snowflake._helpers.cursors import fetch_exact, sub_microsecond_counts
from sqlquack_record.snowflake.classes.snowflake_session import SnowflakeSession
from sqlquack_record.snowflake.models import ConnectionSettings

FAKE_CONFIGURATION: dict[str, Any] = {
    "default_connection_name": "shop",
    "connections": {
        "shop": {
            "account": "example-account",
            "authenticator": "oauth_authorization_code",
            "warehouse": "SMALL",
            "session_parameters": {"TIMEZONE": "America/New_York", "QUERY_TAG": "other"},
        },
        "explicit": {"account": "example-account", "client_store_temporary_credential": False},
    },
}


def arguments_for(
    *,
    monkeypatch: pytest.MonkeyPatch,
    connection_name: str | None,
    warehouse: str | None,
    overrides: tuple[tuple[str, str], ...],
) -> dict[str, object] | None:
    """The connect arguments for case parameters `TIMEZONE=UTC`, or `None` when refused."""
    monkeypatch.setattr(connections, "CONFIG_MANAGER", FAKE_CONFIGURATION)
    settings: ConnectionSettings = ConnectionSettings(
        connection_name=connection_name, warehouse=warehouse, overrides=overrides
    )
    try:
        return connect_arguments(settings=settings, parameters=(("TIMEZONE", "UTC"),))
    except RecorderError:
        return None


_DESCRIPTION: tuple[SimpleNamespace, ...] = (
    SimpleNamespace(
        name="VALUE", type_code=0, precision=38, scale=0, internal_size=None, is_nullable=True
    ),
)


def _succeed() -> None:
    """Executes without error."""


def _raise(error: ProgrammingError) -> None:
    raise error


def _arrow_table() -> pa.Table:
    return pa.table({"VALUE": pa.array([1], type=pa.int64())})


def _truncated_table() -> pa.Table:
    return pa.table({"VALUE": pa.array([0], type=pa.timestamp("us"))})


def _null_table() -> pa.Table:
    return pa.table({"VALUE": pa.array([None], type=pa.timestamp("ns"))})


def _json_result() -> pa.Table:
    raise NotSupportedError


def _overflowing_result() -> pa.Table:
    raise OverflowError("The total number of nanoseconds overflows int64 range.")


_FETCHES: dict[str, Callable[[], pa.Table]] = {
    "arrow": _arrow_table,
    "json": _json_result,
    "overflow": _overflowing_result,
    "truncated": _truncated_table,
    "null": _null_table,
}

_NANOSECOND_DESCRIPTION: tuple[SimpleNamespace, ...] = (
    SimpleNamespace(
        name="VALUE", type_code=8, precision=0, scale=9, internal_size=None, is_nullable=True
    ),
)

_NOT_NULL_DESCRIPTION: tuple[SimpleNamespace, ...] = (
    SimpleNamespace(
        name="VALUE", type_code=8, precision=0, scale=9, internal_size=None, is_nullable=False
    ),
)

_RESULT_DESCRIPTIONS: dict[str, tuple[SimpleNamespace, ...]] = {
    "truncated": _NANOSECOND_DESCRIPTION,
    "null": _NOT_NULL_DESCRIPTION,
}


def _result_kinds(
    *,
    json_results: frozenset[str],
    overflow_results: frozenset[str],
    truncated_results: frozenset[str],
    null_results: frozenset[str],
) -> dict[str, str]:
    return (
        {statement: "json" for statement in json_results}
        | {statement: "overflow" for statement in overflow_results}
        | {statement: "truncated" for statement in truncated_results}
        | {statement: "null" for statement in null_results}
    )


_PARAMETER_DESCRIPTIONS: dict[str, tuple[SimpleNamespace, ...]] = {
    "SHOW PARAMETERS IN SESSION": (SimpleNamespace(name="key"), SimpleNamespace(name="value")),
}


class FakeCursor:
    """Runs each scripted statement and logs it; some answer in JSON, overflow or truncate."""

    def __init__(
        self,
        *,
        behaviours: dict[str, Callable[[], None]],
        result_kinds: dict[str, str],
        log: list[str],
    ) -> None:
        self._behaviours: dict[str, Callable[[], None]] = behaviours
        self._result_kinds: dict[str, str] = result_kinds
        self._log: list[str] = log
        self._results: list[str] = []
        self.sfqid: str = "fake-query"
        self.description: tuple[SimpleNamespace, ...] = _DESCRIPTION

    def execute(self, statement: str, num_statements: int = 1) -> FakeCursor:
        self._log.append(f"{num_statements}:{statement}")
        self._behaviours[statement]()
        self._results = statement.split(";\n")
        self.description = _PARAMETER_DESCRIPTIONS.get(statement, self._current_description())
        return self

    def _current_description(self) -> tuple[SimpleNamespace, ...]:
        kind: str = self._result_kinds.get((*self._results, "")[0], "arrow")
        return _RESULT_DESCRIPTIONS.get(kind, _DESCRIPTION)

    def fetchall(self) -> list[tuple[str, str]]:
        return [("TIMEZONE", "UTC")]

    def fetchone(self) -> tuple[str]:
        return ("0.0.0-fake",)

    def fetch_arrow_all(self, force_return_table: bool) -> pa.Table:
        return _FETCHES[self._result_kinds.get(self._results[0], "arrow")]()

    def nextset(self) -> bool:
        self._results = self._results[1:]
        self.description = self._current_description()
        return bool(self._results)

    def close(self) -> None:
        """Nothing to release."""


class FakeConnection:
    """Hands out cursors that share one script and one log."""

    def __init__(
        self,
        *,
        behaviours: dict[str, Callable[[], None]],
        result_kinds: dict[str, str],
        log: list[str],
    ) -> None:
        self._behaviours: dict[str, Callable[[], None]] = behaviours
        self._result_kinds: dict[str, str] = result_kinds
        self._log: list[str] = log

    def cursor(self) -> FakeCursor:
        return FakeCursor(
            behaviours=self._behaviours, result_kinds=self._result_kinds, log=self._log
        )

    def close(self) -> None:
        """Nothing to release."""


def _error(*, errno: int, sqlstate: str) -> Callable[[], None]:
    return partial(_raise, ProgrammingError(msg="scripted", errno=errno, sqlstate=sqlstate))


def record_with_fake_connection(
    *,
    directory: Path,
    statements: Sequence[str],
    batch_error: tuple[int, str] | None,
    statement_errors: Sequence[tuple[str, int, str]],
    json_results: frozenset[str],
    overflow_results: frozenset[str],
    truncated_results: frozenset[str],
    null_results: frozenset[str],
) -> tuple[tuple[str, ...] | None, tuple[str, ...], int, str]:
    """Outcomes (`None` when the run stopped), single runs, files written and the report."""
    behaviours: defaultdict[str, Callable[[], None]] = defaultdict(lambda: _succeed)
    behaviours.update(
        {
            ";\n".join(statements): _error(errno=errno, sqlstate=sqlstate)
            for errno, sqlstate in filter(None, [batch_error])
        }
    )
    behaviours.update(
        {
            statement: _error(errno=errno, sqlstate=sqlstate)
            for statement, errno, sqlstate in statement_errors
        }
    )
    log: list[str] = []

    def open_session(_: SessionParameters) -> WarehouseSession:
        return SnowflakeSession(
            FakeConnection(
                behaviours=behaviours,
                result_kinds=_result_kinds(
                    json_results=json_results,
                    overflow_results=overflow_results,
                    truncated_results=truncated_results,
                    null_results=null_results,
                ),
                log=log,
            )
        )

    cases: list[PendingCase] = [
        PendingCase(
            case_hash=hashlib.sha256(statement.encode("utf-8")).hexdigest(),
            case_reference=f"fake.toml#{index}",
            source_dialect="snowflake",
            session_parameters=(),
            source_sql=statement,
            duckdb_version="v2.0.0",
        )
        for index, statement in enumerate(statements)
    ]
    results: Path = directory / "results"
    outcomes: tuple[str, ...] | None = None
    message: str = ""
    try:
        summary: RecordSummary = record_pending(
            cases=cases,
            results=results,
            open_session=open_session,
            batch_size=2,
            clock=lambda: "2026-01-01T00:00:00Z",
        )
        outcomes = tuple(_outcome(results / f"{case.case_hash}.json") for case in cases)
        message = "; ".join(f"{reference}: {reason}" for reference, reason in summary.refused)
    except RecorderError as error:
        message = str(error)
    single_runs: frozenset[str] = frozenset(f"1:{statement}" for statement in statements)
    return (
        outcomes,
        tuple(entry.removeprefix("1:") for entry in filter(single_runs.__contains__, log)),
        len(list(results.iterdir())),
        message,
    )


def _stored_outcome(path: Path) -> str:
    return str(json.loads(path.read_text("utf-8"))["outcome"])


def _refused(_: Path) -> str:
    return "refused"


_OUTCOMES: dict[bool, Callable[[Path], str]] = {True: _stored_outcome, False: _refused}


def _outcome(path: Path) -> str:
    return _OUTCOMES[path.exists()](path)


SUB_MICROSECOND_QUERY_ID: str = "01b2c3d4-0000-4a5b-8c9d-0123456789ab"
_SUB_MICROSECOND_TYPE_CODES: dict[str, int] = {
    "TIMESTAMP_LTZ": 6,
    "TIMESTAMP_TZ": 7,
    "TIMESTAMP_NTZ": 8,
    "TIME": 12,
}
_SUB_MICROSECOND_ZONES: dict[str, str | None] = {"TIMESTAMP_LTZ": "UTC", "TIMESTAMP_TZ": "+05:30"}
_COUNTED_POSITION: re.Pattern[str] = re.compile(r"\$(\d+)\)")
_CHECK_TEMPLATE: re.Pattern[str] = re.compile(
    r"SELECT (COUNT_IF\(DATE_PART\(nanosecond, \$\d+\) % 1000 <> 0\)(, )?)+ "
    rf"FROM TABLE\(RESULT_SCAN\('{SUB_MICROSECOND_QUERY_ID}'\)\)"
)


def microsecond_arrow_type(type_name: str) -> pa.DataType:
    """The Arrow type the connector uses for a scale-9 column it downgrades to microseconds."""
    return {"TIME": pa.time64("us")}.get(
        type_name, pa.timestamp("us", tz=_SUB_MICROSECOND_ZONES.get(type_name))
    )


def _sub_microsecond_count(values: tuple[int | None, ...]) -> int:
    present: list[int] = list(filter(lambda value: value is not None, values))
    return sum(value % 1000 != 0 for value in present)


def _micros(value: int | None) -> int | None:
    return value and value // 1000


def _expired() -> None:
    raise ProgrammingError(msg="Result for query has expired", errno=2097, sqlstate="02000")


def _answered() -> None:
    """The check runs."""


class MicrosecondResultCursor:
    """A cursor holding Snowflake's nanosecond values, which the connector hands back in µs."""

    def __init__(
        self,
        *,
        type_name: str,
        names: tuple[str, ...],
        nanoseconds: tuple[tuple[int | None, ...], ...],
    ) -> None:
        self.sfqid: str = SUB_MICROSECOND_QUERY_ID
        self.description: tuple[SimpleNamespace, ...] = tuple(
            SimpleNamespace(
                name=name,
                type_code=_SUB_MICROSECOND_TYPE_CODES[type_name],
                precision=0,
                scale=9,
                internal_size=None,
                is_nullable=True,
            )
            for name in names
        )
        self._columns: tuple[tuple[int | None, ...], ...] = nanoseconds
        self._type: pa.DataType = microsecond_arrow_type(type_name)

    def fetch_arrow_all(self, force_return_table: bool) -> pa.Table:
        arrays: list[pa.Array] = [
            pa.array(list(map(_micros, values)), type=self._type) for values in self._columns
        ]
        return pa.Table.from_arrays(arrays, names=[f"C{i}" for i in range(len(arrays))])


class SubMicrosecondCheckConnection:
    """Answers the RESULT_SCAN check from Snowflake's own values, or as scripted."""

    def __init__(self, *, nanoseconds: tuple[tuple[int | None, ...], ...], answer: str) -> None:
        self._columns: tuple[tuple[int | None, ...], ...] = nanoseconds
        self._answer: str = answer
        self.statements: list[str] = []

    def cursor(self) -> SubMicrosecondCheckConnection:
        return self

    def execute(self, statement: str) -> None:
        self.statements.append(statement)
        {"error": _expired}.get(self._answer, _answered)()

    def fetchall(self) -> list[tuple[object, ...]]:
        counts: list[int] = [
            _sub_microsecond_count(self._columns[int(position) - 1])
            for position in _COUNTED_POSITION.findall(self.statements[-1])
        ]
        answers: dict[str, list[tuple[object, ...]]] = {
            "counts": [tuple(counts)],
            "two rows": [tuple(counts), tuple(counts)],
            "null": [(None,) * len(counts)],
            "boolean": [(False,) * len(counts)],
            "decimal": [tuple(Decimal(count) for count in counts)],
            "fraction": [(Decimal("0.5"),) * len(counts)],
        }
        return answers[self._answer]

    def close(self) -> None:
        """Nothing to release."""


def checked_or_refused(
    *,
    type_name: str,
    names: tuple[str, ...],
    nanoseconds: tuple[tuple[int | None, ...], ...],
    answer: str,
) -> tuple[tuple[bool, ...] | None, str | None, int]:
    """Columns marked checked, or the refusal, and how many check statements ran."""
    connection: SubMicrosecondCheckConnection = SubMicrosecondCheckConnection(
        nanoseconds=nanoseconds, answer=answer
    )
    cursor: MicrosecondResultCursor = MicrosecondResultCursor(
        type_name=type_name, names=names, nanoseconds=nanoseconds
    )
    checks: dict[str, SubMicrosecondCheck | None] = {"absent": None}
    try:
        result: StatementResult = fetch_exact(
            cursor=cursor,
            exact=checks.get(answer, partial(sub_microsecond_counts, connection=connection)),
        )
    except UnrecordableResultError as error:
        return None, str(error), len(connection.statements)
    assert all(_CHECK_TEMPLATE.fullmatch(statement) for statement in connection.statements)
    assert result.table is not None
    assert all(field.type == microsecond_arrow_type(type_name) for field in result.table.schema)
    checked: tuple[bool, ...] = tuple(column.sub_microsecond_checked for column in result.columns)
    return checked, None, len(connection.statements)
