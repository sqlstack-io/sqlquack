"""Raising value tuples found in one pass: raises observed by a hook, or counted with TRY."""

from __future__ import annotations

import re

import duckdb
import duckdb.sqltypes
from duckdb.func import FunctionNullHandling, PythonUDFType

from scripts.guard_measurement.classes.raise_log import RaiseLog
from scripts.guard_measurement.constants import (
    BODY_END,
    BODY_START,
    COMMON_SUBEXPRESSIONS,
    RAISE_HOOK,
    RAISE_SITE,
    RESULT_PROJECTION,
    SAMPLE_RELATION,
    SELECT_PREFIX,
)


def probe_expression(*, body: str) -> str | None:
    """The per-row expression of an emitted probe macro, or None for an unexpected shape."""
    start: int = body.find(BODY_START)
    end: int = body.rfind(BODY_END)
    if start < 0 or end <= start:
        return None
    inner: str = body[start + len(BODY_START) : end]
    cut: int = inner.rfind(RESULT_PROJECTION)
    if not inner.startswith(SELECT_PREFIX) or cut < 0:
        return None
    return inner[len(SELECT_PREFIX) : cut]


def hooked_raises(
    *, connection: duckdb.DuckDBPyConnection, expression: str, projected: str
) -> dict[int, str] | None:
    """Each raising tuple's first raise message, or None when DuckDB raises an error of its own."""
    log: RaiseLog = RaiseLog()
    connection.create_function(
        RAISE_HOOK,
        log,
        [duckdb.sqltypes.VARCHAR, duckdb.sqltypes.BIGINT],
        duckdb.sqltypes.INTEGER,
        type=PythonUDFType.NATIVE,
        null_handling=FunctionNullHandling.SPECIAL,
        side_effects=True,
    )
    hooked: str = RAISE_SITE.sub(_hook_call, expression)
    try:
        _without_common_subexpressions(
            connection=connection,
            query=f"SELECT count(*) FROM (SELECT __idx, {projected} FROM tuples) AS "
            f'"{SAMPLE_RELATION}" WHERE ({hooked}) IS NOT NULL',
        )
    except duckdb.Error:
        return None
    finally:
        connection.remove_function(RAISE_HOOK)
    return log.first


def raising_tuples(
    *, connection: duckdb.DuckDBPyConnection, expression: str, projected: str
) -> list[int] | None:
    """Tuple numbers on which `expression` raises, or None when DuckDB cannot TRY it."""
    try:
        rows: list[tuple[int]] = _without_common_subexpressions(
            connection=connection,
            query=f"SELECT __idx FROM (SELECT __idx, {projected} FROM tuples) AS "
            f'"{SAMPLE_RELATION}" WHERE TRY({expression}) IS NULL ORDER BY __idx',
        )
    except duckdb.Error:
        return None
    return [row[0] for row in rows]


def _hook_call(site: re.Match[str]) -> str:
    """A raise site calling the hook with its message and the tuple number, returning NULL."""
    return f'CAST({RAISE_HOOK}(\'{site.group(1)}\', "{SAMPLE_RELATION}"."__idx") AS "NULL")'


def _without_common_subexpressions(
    *, connection: duckdb.DuckDBPyConnection, query: str
) -> list[tuple[int]]:
    """`query`'s rows; common-subexpression elimination would evaluate guards outside their CASE."""
    previous: tuple[str] | None = connection.execute(
        "SELECT current_setting('disabled_optimizers')"
    ).fetchone()
    connection.execute(f"SET disabled_optimizers = '{COMMON_SUBEXPRESSIONS}'")
    try:
        return connection.execute(query).fetchall()
    finally:
        connection.execute(f"SET disabled_optimizers = '{previous[0] if previous else ''}'")
