"""A Snowflake connection driven by the recorder."""

from __future__ import annotations

from collections.abc import Sequence
from importlib.metadata import version
from typing import Any

from snowflake.connector.errors import ProgrammingError

from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.models import StatementResult, WarehouseFailure
from sqlquack_record.snowflake._helpers.cursors import fetch, require_recordable


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
        """Run statements as one multi-statement request."""
        cursor: Any = self._connection.cursor()
        try:
            cursor.execute(";\n".join(statements), num_statements=len(statements))
            results: list[StatementResult] = [fetch(cursor)]
            for _ in statements[1:]:
                if not cursor.nextset():
                    raise RecorderError("the warehouse returned fewer results than statements")
                results.append(fetch(cursor))
        except ProgrammingError as error:
            require_recordable(error)
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
        except ProgrammingError as error:
            require_recordable(error)
            return StatementResult(
                query_id=error.sfqid,
                columns=(),
                table=None,
                failure=WarehouseFailure(
                    errno=error.errno, sqlstate=error.sqlstate, message=str(error.msg)
                ),
            )
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
