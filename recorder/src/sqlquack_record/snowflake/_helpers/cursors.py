"""Reading connector cursors without converting values."""

from __future__ import annotations

from typing import Any

from snowflake.connector.constants import FIELD_ID_TO_NAME
from snowflake.connector.errors import ProgrammingError

from sqlquack_record.recording.constants import RECORDABLE_SQLSTATE_CLASSES
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.models import ColumnDescription, StatementResult


def fetch(cursor: Any) -> StatementResult:
    """The cursor's current result as the connector's Arrow table."""
    return StatementResult(
        query_id=cursor.sfqid,
        columns=tuple(_describe(column) for column in cursor.description),
        table=cursor.fetch_arrow_all(force_return_table=True),
        failure=None,
    )


def require_recordable(error: ProgrammingError) -> None:
    """Raise unless `error` is a SQL error that belongs to the case rather than the session."""
    sqlstate: str = str(error.sqlstate or "")
    if sqlstate[:2] not in RECORDABLE_SQLSTATE_CLASSES:
        raise RecorderError(
            f"Snowflake error {error.errno} (SQLSTATE {sqlstate or 'unknown'}) is not a case "
            f"outcome; nothing was recorded: {error.msg}"
        ) from error


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
