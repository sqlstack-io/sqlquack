"""A Snowflake connection driven by the recorder."""

from __future__ import annotations

from collections.abc import Sequence
from importlib.metadata import version
from typing import Any

from snowflake.connector.errors import ProgrammingError

from sqlquack_record.recording.constants import (
    PARAMETER_KEY_COLUMN,
    PARAMETER_VALUE_COLUMN,
    SHOW_SESSION_PARAMETERS,
)
from sqlquack_record.recording.exceptions import RecorderError, UnrecordableResultError
from sqlquack_record.recording.models import StatementResult, WarehouseFailure
from sqlquack_record.snowflake._helpers.cursors import (
    error_message,
    fetch,
    require_batch_recordable,
    require_recordable,
)


class SnowflakeSession:
    """Runs case SQL and returns the connector's Arrow results untouched."""

    def __init__(self, connection: Any) -> None:
        """Wrap an open connection."""
        self._connection: Any = connection

    def warehouse_version(self) -> str:
        """`CURRENT_VERSION()`."""
        return str(self._scalar("SELECT CURRENT_VERSION()"))

    def connector(self) -> str:
        """The installed connector name and version."""
        return f"snowflake-connector-python {version('snowflake-connector-python')}"

    def current_warehouse(self) -> str | None:
        """`CURRENT_WAREHOUSE()`, absent when none is in use."""
        value: object = self._scalar("SELECT CURRENT_WAREHOUSE()")
        return None if value is None else str(value)

    def run_batch(self, statements: Sequence[str]) -> list[StatementResult] | None:
        """Run statements as one request; `None` asks for a rerun one statement at a time."""
        cursor: Any = self._connection.cursor()
        try:
            cursor.execute(";\n".join(statements), num_statements=len(statements))
            results: list[StatementResult] = [fetch(cursor)]
            for _ in statements[1:]:
                if not cursor.nextset():
                    raise RecorderError("the warehouse returned fewer results than statements")
                results.append(fetch(cursor))
        except ProgrammingError as error:
            require_batch_recordable(error)
            return None
        except UnrecordableResultError:
            return None
        finally:
            cursor.close()
        return results

    def run_one(self, statement: str) -> StatementResult:
        """Run one statement; a recordable SQL error becomes the result."""
        cursor: Any = self._connection.cursor()
        try:
            cursor.execute(statement)
            return fetch(cursor)
        except UnrecordableResultError as error:
            return StatementResult(
                query_id=cursor.sfqid, columns=(), table=None, failure=None, refusal=str(error)
            )
        except ProgrammingError as error:
            require_recordable(error)
            return StatementResult(
                query_id=error.sfqid,
                columns=(),
                table=None,
                failure=WarehouseFailure(
                    errno=error.errno, sqlstate=error.sqlstate, message=error_message(error)
                ),
            )
        finally:
            cursor.close()

    def execute(self, statement: str) -> None:
        """Run a statement that is not a case; a warehouse error raises `RecorderError`."""
        cursor: Any = self._connection.cursor()
        try:
            cursor.execute(statement)
        except ProgrammingError as error:
            raise RecorderError(f"setup statement failed: {error_message(error)}") from error
        finally:
            cursor.close()

    def current_schema(self) -> str | None:
        """`CURRENT_SCHEMA()`, absent when none is in use."""
        value: object = self._scalar("SELECT CURRENT_SCHEMA()")
        return None if value is None else str(value)

    def session_parameters(self) -> dict[str, str]:
        """Every parameter's value as `SHOW PARAMETERS IN SESSION` reports it, by name."""
        cursor: Any = self._connection.cursor()
        try:
            cursor.execute(SHOW_SESSION_PARAMETERS)
            names: list[str] = [str(column.name).lower() for column in cursor.description]
            key: int = names.index(PARAMETER_KEY_COLUMN)
            value: int = names.index(PARAMETER_VALUE_COLUMN)
            return {str(row[key]).upper(): str(row[value]) for row in cursor.fetchall()}
        finally:
            cursor.close()

    def close(self) -> None:
        """Close the connection."""
        self._connection.close()

    def _scalar(self, statement: str) -> object:
        cursor: Any = self._connection.cursor()
        try:
            row: tuple[object, ...] | None = cursor.execute(statement).fetchone()
        finally:
            cursor.close()
        return None if row is None else row[0]
