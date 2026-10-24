"""A Snowflake connection driven by the recorder."""

from __future__ import annotations

from collections.abc import Sequence
from importlib.metadata import version
from typing import Any

from snowflake.connector.constants import FIELD_ID_TO_NAME
from snowflake.connector.errors import ProgrammingError

from sqlquack_record.constants import RECORDABLE_SQLSTATE_CLASSES
from sqlquack_record.errors import RecorderError
from sqlquack_record.models import ColumnDescription, RecordedError, StatementResult


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
            results: list[StatementResult] = [_fetch(cursor)]
            for _ in statements[1:]:
                if not cursor.nextset():
                    raise RecorderError("the warehouse returned fewer results than statements")
                results.append(_fetch(cursor))
        except ProgrammingError as error:
            _require_recordable(error)
            return None
        finally:
            cursor.close()
        return results

    def run_one(self, statement: str) -> StatementResult:
        """Run one statement; a recordable SQL error becomes the result."""
        cursor: Any = self._connection.cursor()
        try:
            cursor.execute(statement)
            return _fetch(cursor)
        except ProgrammingError as error:
            _require_recordable(error)
            return StatementResult(
                query_id=error.sfqid,
                columns=(),
                table=None,
                error=RecordedError(
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


def _fetch(cursor: Any) -> StatementResult:
    table: Any = cursor.fetch_arrow_all(force_return_table=True)
    return StatementResult(
        query_id=cursor.sfqid,
        columns=tuple(_describe(column) for column in cursor.description),
        table=table,
        error=None,
    )


def _describe(column: Any) -> ColumnDescription:
    return ColumnDescription(
        name=str(column.name),
        type_code=int(column.type_code),
        type_name=str(FIELD_ID_TO_NAME.get(column.type_code, column.type_code)),
        precision=column.precision,
        scale=column.scale,
        internal_size=column.internal_size,
        nullable=column.is_nullable,
    )


def _require_recordable(error: ProgrammingError) -> None:
    sqlstate: str = str(error.sqlstate or "")
    if sqlstate[:2] not in RECORDABLE_SQLSTATE_CLASSES:
        raise RecorderError(
            f"Snowflake error {error.errno} (SQLSTATE {sqlstate or 'unknown'}) is not a case "
            f"outcome; nothing was recorded: {error.msg}"
        ) from error
