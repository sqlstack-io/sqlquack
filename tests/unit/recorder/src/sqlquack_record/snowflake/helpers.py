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
from snowflake.connector.errors import ProgrammingError

from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.main.record_pending import record_pending
from sqlquack_record.recording.models import PendingCase
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


class FakeCursor:
    """Runs each statement by its scripted behaviour and logs it."""

    def __init__(self, *, behaviours: dict[str, Callable[[], None]], log: list[str]) -> None:
        self._behaviours: dict[str, Callable[[], None]] = behaviours
        self._log: list[str] = log
        self.sfqid: str = "fake-query"
        self.description: tuple[SimpleNamespace, ...] = _DESCRIPTION

    def execute(self, statement: str, num_statements: int = 1) -> FakeCursor:
        self._log.append(f"{num_statements}:{statement}")
        self._behaviours[statement]()
        return self

    def fetchone(self) -> tuple[str]:
        return ("0.0.0-fake",)

    def fetch_arrow_all(self, force_return_table: bool) -> pa.Table:
        return pa.table({"VALUE": pa.array([1], type=pa.int64())})

    def nextset(self) -> bool:
        return True

    def close(self) -> None:
        """Nothing to release."""


class FakeConnection:
    """Hands out cursors that share one script and one log."""

    def __init__(self, *, behaviours: dict[str, Callable[[], None]], log: list[str]) -> None:
        self._behaviours: dict[str, Callable[[], None]] = behaviours
        self._log: list[str] = log

    def cursor(self) -> FakeCursor:
        return FakeCursor(behaviours=self._behaviours, log=self._log)

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
) -> tuple[tuple[str, ...] | None, tuple[str, ...], int]:
    """Recorded outcomes (`None` when the run stopped), single-statement runs and files written."""
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
        return SnowflakeSession(FakeConnection(behaviours=behaviours, log=log))

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
    try:
        record_pending(
            cases=cases,
            results=results,
            open_session=open_session,
            batch_size=10,
            clock=lambda: "2026-01-01T00:00:00Z",
        )
        outcomes = tuple(
            str(json.loads((results / f"{case.case_hash}.json").read_text("utf-8"))["outcome"])
            for case in cases
        )
    except RecorderError:
        pass
    single_runs: frozenset[str] = frozenset(f"1:{statement}" for statement in statements)
    return (
        outcomes,
        tuple(entry.removeprefix("1:") for entry in filter(single_runs.__contains__, log)),
        len(list(results.iterdir())),
    )
