"""Connection arguments against a fake configuration, and a fake connection for sessions."""

from __future__ import annotations

import hashlib
import json
from collections import defaultdict
from collections.abc import Callable, Sequence
from functools import partial
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pyarrow as pa
import pytest
from snowflake.connector.errors import NotSupportedError, ProgrammingError

from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.main.record_pending import record_pending
from sqlquack_record.recording.models import PendingCase, RecordSummary
from sqlquack_record.recording.types import SessionParameters, WarehouseSession
from sqlquack_record.snowflake._helpers import connections
from sqlquack_record.snowflake._helpers.connections import connect_arguments
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


def _json_result() -> pa.Table:
    raise NotSupportedError


_FETCHES: dict[bool, Callable[[], pa.Table]] = {False: _arrow_table, True: _json_result}


_PARAMETER_DESCRIPTIONS: dict[str, tuple[SimpleNamespace, ...]] = {
    "SHOW PARAMETERS IN SESSION": (SimpleNamespace(name="key"), SimpleNamespace(name="value")),
}


class FakeCursor:
    """Runs each statement by its scripted behaviour and logs it; some answer in JSON."""

    def __init__(
        self,
        *,
        behaviours: dict[str, Callable[[], None]],
        json_results: frozenset[str],
        log: list[str],
    ) -> None:
        self._behaviours: dict[str, Callable[[], None]] = behaviours
        self._json_results: frozenset[str] = json_results
        self._log: list[str] = log
        self._results: list[str] = []
        self.sfqid: str = "fake-query"
        self.description: tuple[SimpleNamespace, ...] = _DESCRIPTION

    def execute(self, statement: str, num_statements: int = 1) -> FakeCursor:
        self._log.append(f"{num_statements}:{statement}")
        self._behaviours[statement]()
        self._results = statement.split(";\n")
        self.description = _PARAMETER_DESCRIPTIONS.get(statement, _DESCRIPTION)
        return self

    def fetchall(self) -> list[tuple[str, str]]:
        return [("TIMEZONE", "UTC")]

    def fetchone(self) -> tuple[str]:
        return ("0.0.0-fake",)

    def fetch_arrow_all(self, force_return_table: bool) -> pa.Table:
        return _FETCHES[self._results[0] in self._json_results]()

    def nextset(self) -> bool:
        self._results = self._results[1:]
        return bool(self._results)

    def close(self) -> None:
        """Nothing to release."""


class FakeConnection:
    """Hands out cursors that share one script and one log."""

    def __init__(
        self,
        *,
        behaviours: dict[str, Callable[[], None]],
        json_results: frozenset[str],
        log: list[str],
    ) -> None:
        self._behaviours: dict[str, Callable[[], None]] = behaviours
        self._json_results: frozenset[str] = json_results
        self._log: list[str] = log

    def cursor(self) -> FakeCursor:
        return FakeCursor(
            behaviours=self._behaviours, json_results=self._json_results, log=self._log
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
            FakeConnection(behaviours=behaviours, json_results=json_results, log=log)
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
