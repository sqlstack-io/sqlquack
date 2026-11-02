"""Reading connector cursors without converting values."""

from __future__ import annotations

from typing import Any

import pyarrow as pa
from snowflake.connector.constants import FIELD_ID_TO_NAME
from snowflake.connector.errors import NotSupportedError, ProgrammingError

from sqlquack_record.recording.constants import (
    DATA_EXCEPTION_SQLSTATE_CLASSES,
    ERROR_PREFIX_PATTERN,
    INTERNAL_SQLSTATE_CLASS,
    MICROSECOND_SCALE,
    MULTI_STATEMENT_FAILURE,
    NANOSECOND_TYPE_NAMES,
    NANOSECOND_UNIT,
    QUERY_ID_PATTERN,
    RECORDABLE_FUNCTION_ERRORS,
    RECORDABLE_SQLSTATE_CLASSES,
    SESSION_DEPENDENT_SQLSTATES,
    SESSION_FAILURE_SQLSTATE_CLASSES,
)
from sqlquack_record.recording.exceptions import (
    InternalCompileError,
    RecorderError,
    StatementNotRecordableError,
    UnrecordableResultError,
)
from sqlquack_record.recording.models import ColumnDescription, StatementResult


def fetch(cursor: Any) -> StatementResult:
    """The cursor's current result as the connector's Arrow table."""
    columns: tuple[ColumnDescription, ...] = tuple(
        describe_column(column) for column in cursor.description or ()
    )
    try:
        table: pa.Table | None = cursor.fetch_arrow_all(force_return_table=True)
    except NotSupportedError as error:
        raise UnrecordableResultError(
            "Snowflake returned the result in JSON rather than Arrow, so its exact types "
            "cannot be kept"
        ) from error
    except OverflowError as error:
        raise UnrecordableResultError(
            f"the connector cannot convert the result exactly: {error}"
        ) from error
    if table is None:
        raise UnrecordableResultError("the connector returned no Arrow table for the result")
    _require_declared_units(columns=columns, table=table)
    _require_declared_nullability(columns=columns, table=table)
    return StatementResult(query_id=cursor.sfqid, columns=columns, table=table, failure=None)


def require_recordable(error: ProgrammingError) -> None:
    """Raise unless `error` is a SQL error that belongs to the case rather than the session."""
    sqlstate: str = str(error.sqlstate or "")
    if sqlstate[:2] in RECORDABLE_SQLSTATE_CLASSES and sqlstate not in SESSION_DEPENDENT_SQLSTATES:
        return
    if (error.errno, sqlstate) in RECORDABLE_FUNCTION_ERRORS:
        return
    message: str = (
        f"Snowflake error {error.errno} (SQLSTATE {sqlstate or 'unknown'}) is not a case "
        f"outcome; nothing was recorded: {error.msg}"
    )
    if not sqlstate or sqlstate[:2] in SESSION_FAILURE_SQLSTATE_CLASSES:
        raise RecorderError(message) from error
    raise StatementNotRecordableError(message) from error


def error_message(error: ProgrammingError) -> str | None:
    """A data exception's message without its errno and SQLSTATE prefix or any query ID."""
    if str(error.sqlstate or "")[:2] not in DATA_EXCEPTION_SQLSTATE_CLASSES:
        return None
    message: str = QUERY_ID_PATTERN.sub("", str(error.msg or ""))
    if error.sfqid:
        message = message.replace(str(error.sfqid), "")
    return ERROR_PREFIX_PATTERN.sub("", message).strip(" :")


def compile_message(error: ProgrammingError) -> str:
    """A compile error's message for the console: session failures raise, no query ID is kept."""
    sqlstate: str = str(error.sqlstate or "")
    if sqlstate[:2] == INTERNAL_SQLSTATE_CLASS:
        raise InternalCompileError(
            f"Snowflake error {error.errno} (SQLSTATE {sqlstate}) while compiling a case; "
            f"nothing was recorded: {error.msg}"
        ) from error
    if not sqlstate or sqlstate[:2] in SESSION_FAILURE_SQLSTATE_CLASSES:
        raise RecorderError(
            f"Snowflake error {error.errno} (SQLSTATE {sqlstate or 'unknown'}) while compiling "
            f"a case; nothing was recorded: {error.msg}"
        ) from error
    message: str = QUERY_ID_PATTERN.sub("", str(error.msg or ""))
    if error.sfqid:
        message = message.replace(str(error.sfqid), "")
    return " ".join(ERROR_PREFIX_PATTERN.sub("", message).split())


def require_batch_recordable(error: ProgrammingError) -> None:
    """Accept a multi-statement wrapper error as a batch failure; otherwise `require_recordable`."""
    if (error.errno, error.sqlstate) != MULTI_STATEMENT_FAILURE:
        require_recordable(error)


def _require_declared_units(*, columns: tuple[ColumnDescription, ...], table: pa.Table) -> None:
    """Refuse a result whose sub-microsecond column the connector truncated to microseconds."""
    for index, column in enumerate(columns):
        unit: str | None = getattr(table.schema.field(index).type, "unit", None)
        if (
            column.type_name in NANOSECOND_TYPE_NAMES
            and (column.scale or 0) > MICROSECOND_SCALE
            and unit not in (None, NANOSECOND_UNIT)
        ):
            raise UnrecordableResultError(
                f"the connector returned {column.name} {column.type_name}({column.scale}) in "
                f"{unit} rather than ns, which truncates every row's sub-microsecond digits "
                "when any row lies outside the nanosecond range"
            )


def _require_declared_nullability(
    *, columns: tuple[ColumnDescription, ...], table: pa.Table
) -> None:
    """Refuse a NULL in a NOT NULL column: the connector's, standing for a value no one knows."""
    for index, column in enumerate(columns):
        if column.nullable is False and table.column(index).null_count > 0:
            raise UnrecordableResultError(
                f"the connector returned NULL in {column.name} {column.type_name}, which "
                "Snowflake declares NOT NULL, so the value it stands for is unknown"
            )


def describe_column(column: Any) -> ColumnDescription:
    """One column of a cursor description or a described statement."""
    return ColumnDescription(
        name=str(column.name),
        type_code=int(column.type_code),
        type_name=str(FIELD_ID_TO_NAME.get(column.type_code, column.type_code)),
        precision=column.precision,
        scale=column.scale,
        internal_size=column.internal_size,
        nullable=column.is_nullable,
    )
