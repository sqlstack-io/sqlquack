"""Reading connector cursors without converting values."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import replace
from decimal import Decimal
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
    NANOS_PER_MICRO,
    NANOSECOND_TYPE_NAMES,
    NANOSECOND_UNIT,
    QUERY_ID_CHARACTERS,
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
from sqlquack_record.recording.types import SubMicrosecondCheck


def fetch(cursor: Any) -> StatementResult:
    """The cursor's current result as the connector's Arrow table; µs-held ns columns refuse."""
    return fetch_exact(cursor=cursor, exact=None)


def fetch_exact(*, cursor: Any, exact: SubMicrosecondCheck | None) -> StatementResult:
    """The cursor's current result; a µs-held ns column stays only if `exact` proves it whole."""
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
    columns = _require_exact_units(columns=columns, table=table, query_id=cursor.sfqid, exact=exact)
    _require_declared_nullability(columns=columns, table=table)
    return StatementResult(query_id=cursor.sfqid, columns=columns, table=table, failure=None)


def sub_microsecond_counts(
    *, connection: Any, query_id: str, positions: Sequence[int]
) -> tuple[int, ...]:
    """Per 0-based position, values with sub-µs digits: one SELECT over the RESULT_SCAN."""
    if not query_id or not set(query_id) <= QUERY_ID_CHARACTERS or not positions:
        raise UnrecordableResultError(
            "the sub-microsecond check has no query ID or no column to count"
        )
    counts: str = ", ".join(
        f"COUNT_IF(DATE_PART(nanosecond, ${position + 1}) % {NANOS_PER_MICRO} <> 0)"
        for position in positions
    )
    cursor: Any = connection.cursor()
    try:
        cursor.execute(f"SELECT {counts} FROM TABLE(RESULT_SCAN('{query_id}'))")
        rows: list[tuple[Any, ...]] = list(cursor.fetchall())
    except ProgrammingError as error:
        raise UnrecordableResultError(
            f"the sub-microsecond check failed, so nothing was recorded: {error.msg}"
        ) from error
    finally:
        cursor.close()
    if len(rows) != 1 or len(rows[0]) != len(positions):
        raise UnrecordableResultError(
            f"the sub-microsecond check returned {len(rows)} rows rather than one row of "
            f"{len(positions)} counts"
        )
    values: tuple[Any, ...] = tuple(rows[0])
    counts: tuple[int | None, ...] = tuple(_whole_count(value) for value in values)
    if None in counts:
        raise UnrecordableResultError(
            f"the sub-microsecond check returned {values!r} rather than integer counts"
        )
    return tuple(count for count in counts if count is not None)


def _whole_count(value: object) -> int | None:
    """A count as an int, from an int or a whole Decimal; None for anything else."""
    if type(value) is int:
        return value
    if type(value) is Decimal and value.is_finite() and value == value.to_integral_value():
        return int(value)
    return None


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


def _require_exact_units(
    *,
    columns: tuple[ColumnDescription, ...],
    table: pa.Table,
    query_id: str | None,
    exact: SubMicrosecondCheck | None,
) -> tuple[ColumnDescription, ...]:
    """The columns, marking µs-held ns columns `exact` proves whole; anything else refuses."""
    downgraded: list[int] = [
        index
        for index, column in enumerate(columns)
        if column.type_name in NANOSECOND_TYPE_NAMES
        and (column.scale or 0) > MICROSECOND_SCALE
        and getattr(table.schema.field(index).type, "unit", None) not in (None, NANOSECOND_UNIT)
    ]
    if not downgraded:
        return columns
    first: ColumnDescription = columns[downgraded[0]]
    unit: object = getattr(table.schema.field(downgraded[0]).type, "unit", None)
    truncating: str = (
        f"the connector returned {first.name} {first.type_name}({first.scale}) in {unit} rather "
        "than ns, which truncates every row's sub-microsecond digits"
    )
    if exact is None or query_id is None:
        raise UnrecordableResultError(
            f"{truncating} when any row lies outside the nanosecond range"
        )
    try:
        counts: tuple[int, ...] = exact(query_id=query_id, positions=downgraded)
    except UnrecordableResultError as error:
        raise UnrecordableResultError(f"{truncating}; {error}") from error
    for index, count in zip(downgraded, counts, strict=True):
        if count != 0:
            column: ColumnDescription = columns[index]
            raise UnrecordableResultError(
                f"the connector returned {column.name} {column.type_name}({column.scale}) in "
                f"microseconds, and {count} of its values have sub-microsecond digits that "
                "unit would drop"
            )
    return tuple(
        replace(column, sub_microsecond_checked=True) if index in downgraded else column
        for index, column in enumerate(columns)
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
