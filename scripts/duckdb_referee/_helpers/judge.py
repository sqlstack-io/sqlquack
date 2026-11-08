"""Judge one case in a fresh in-memory DuckDB database: parse it alone, then run it."""

from __future__ import annotations

from collections.abc import Callable

import duckdb

from scripts.duckdb_referee.constants import (
    ACCEPTED,
    BASE_ERROR,
    BASIS_BIND,
    BASIS_EXECUTE,
    BASIS_PARSE,
    BASIS_RAN,
    ENGINE_CODES,
    EXPLAIN,
    EXTENSION_LOAD_CODE,
    EXTENSION_LOAD_PREFIX,
    EXTERNAL_ACCESS_DISABLED,
    PARSER_ERROR,
    PERMISSION_CODE,
    PHASES,
    QUERY_CONTEXT,
    RUNTIME_ERROR,
    SEMANTIC_ERROR,
    SYNTAX_ERROR,
    UNSUPPORTED_FEATURE,
)
from scripts.duckdb_referee.exceptions import RefereeError
from scripts.duckdb_referee.models import Observation


def code_of(error: Exception) -> tuple[str, str]:
    """The engine code and phase of a DuckDB error; an unmapped type stops the run."""
    name: str = type(error).__name__
    if name == BASE_ERROR and str(error).startswith(EXTENSION_LOAD_PREFIX):
        if EXTERNAL_ACCESS_DISABLED in str(error):
            return PERMISSION_CODE, RUNTIME_ERROR
        return EXTENSION_LOAD_CODE, UNSUPPORTED_FEATURE
    if name not in ENGINE_CODES:
        raise RefereeError(f"no engine code for {name}: {error}")
    return ENGINE_CODES[name], PHASES[name]


def message_of(error: Exception) -> str:
    """DuckDB's exception name and message, verbatim."""
    return f"{type(error).__name__}: {error}"


def without_context(message: str) -> str:
    """The message without the `LINE n:` excerpt of the query text that DuckDB may append."""
    return message.split(QUERY_CONTEXT, 1)[0]


def run_to_end(*, connection: duckdb.DuckDBPyConnection, text: str) -> None:
    """Run one statement until the engine has produced every result row, converting none."""
    connection.sql(text).materialize()


def prepared(*, session: dict[str, str], setup: list[str]) -> duckdb.DuckDBPyConnection:
    """A fresh in-memory database with the case's session settings and setup applied."""
    connection: duckdb.DuckDBPyConnection = duckdb.connect(":memory:")
    texts: list[str] = [f"SET {name} = {value}" for name, value in sorted(session.items())]
    for text in texts + setup:
        try:
            run_to_end(connection=connection, text=text)
        except duckdb.Error as error:
            connection.close()
            raise RefereeError(f"setup statement {text!r} failed: {message_of(error)}") from error
    return connection


def observe(
    *,
    sql: str,
    session: dict[str, str],
    setup: list[str],
    set_up: Callable[[], None],
    parsed: Callable[[int], None],
) -> Observation:
    """The case's phase: only the parse-only pass decides syntax; execution decides the rest."""
    connection: duckdb.DuckDBPyConnection = prepared(session=session, setup=setup)
    set_up()
    try:
        statements: list[str] = [s.query for s in connection.extract_statements(sql)]
    except duckdb.Error as error:
        code, phase = code_of(error)
        connection.close()
        return Observation(
            phase=phase,
            basis=BASIS_PARSE.format(name=type(error).__name__),
            engine_code=code,
            parse_only_code=code,
            message=message_of(error),
            statements=0,
        )
    if not statements:
        raise RefereeError(f"no statement to judge in {sql!r}")
    parsed(len(statements))
    for index, statement in enumerate(statements):
        try:
            run_to_end(connection=connection, text=statement)
        except duckdb.Error as error:
            connection.close()
            return _failed(
                error=error,
                statements=statements,
                index=index,
                context=(session, setup),
            )
    connection.close()
    return Observation(
        phase=ACCEPTED,
        basis=BASIS_RAN,
        engine_code=None,
        parse_only_code=None,
        message=None,
        statements=len(statements),
    )


def _failed(
    *,
    error: duckdb.Error,
    statements: list[str],
    index: int,
    context: tuple[dict[str, str], list[str]],
) -> Observation:
    """A statement's execution error; a parser error is semantic when EXPLAIN of it raises it."""
    code, phase = code_of(error)
    basis: str = BASIS_EXECUTE.format(name=type(error).__name__)
    if code == ENGINE_CODES[PARSER_ERROR] and _explain_raises(
        message=message_of(error), statements=statements, index=index, context=context
    ):
        phase, basis = SEMANTIC_ERROR, BASIS_BIND.format(name=type(error).__name__)
    return Observation(
        phase=RUNTIME_ERROR if phase == SYNTAX_ERROR else phase,
        basis=basis,
        engine_code=code,
        parse_only_code=None,
        message=message_of(error),
        statements=len(statements),
    )


def _explain_raises(
    *,
    message: str,
    statements: list[str],
    index: int,
    context: tuple[dict[str, str], list[str]],
) -> bool:
    """Whether EXPLAIN of the failing statement alone, after those before it, raises the message."""
    session, setup = context
    connection: duckdb.DuckDBPyConnection = prepared(session=session, setup=setup)
    try:
        for statement in statements[:index]:
            run_to_end(connection=connection, text=statement)
        run_to_end(connection=connection, text=EXPLAIN + statements[index])
    except duckdb.Error as error:
        return without_context(message_of(error)) == without_context(message)
    finally:
        connection.close()
    return False
