"""Loading DuckDB's extensions and probing the names its parser handles itself."""

from __future__ import annotations

import duckdb

from scripts.duckdb_catalogue.constants import (
    PARSER_KEYWORDS_QUERY,
    PROBE_ARGUMENTS,
    SYNTAX_FORM_PROBES,
    SYNTAX_MARKER,
    TRANSFORMER_REWRITTEN_NAMES,
    UNKNOWN_FUNCTION_MARKERS,
    WINDOW_KIND,
    WINDOW_PROBE,
)


def load_extensions(connection: duckdb.DuckDBPyConnection) -> tuple[list[str], list[str]]:
    """Install and load every extension DuckDB lists; return those loaded and those that fail."""
    loaded: list[str] = []
    unloadable: list[str] = []
    for (name,) in connection.execute("SELECT extension_name FROM duckdb_extensions()").fetchall():
        try:
            connection.execute(f"INSTALL {name}")
            connection.execute(f"LOAD {name}")
        except duckdb.Error:
            unloadable.append(name)
        else:
            loaded.append(name)
    return loaded, unloadable


def special_forms(*, connection: duckdb.DuckDBPyConnection, known: set[str]) -> list[str]:
    """Keywords and transformed names whose call DuckDB does not refuse as an unknown function."""
    keywords: list[str] = [row[0] for row in connection.execute(PARSER_KEYWORDS_QUERY).fetchall()]
    special: list[str] = []
    for name in sorted({*keywords, *TRANSFORMER_REWRITTEN_NAMES} - known):
        if any(
            reaches_past_lookup(connection=connection, name=name, arguments=arguments)
            for arguments in PROBE_ARGUMENTS
        ):
            special.append(name)
    return special


def syntax_forms(connection: duckdb.DuckDBPyConnection) -> list[str]:
    """Names the parser emits as calls for its own syntax, kept where DuckDB runs that syntax."""
    forms: list[str] = []
    for name, probe in SYNTAX_FORM_PROBES.items():
        try:
            connection.execute(probe)
        except duckdb.Error:
            continue
        forms.append(name)
    return forms


def reaches_past_lookup(
    *, connection: duckdb.DuckDBPyConnection, name: str, arguments: str
) -> bool:
    """Whether calling `name` neither fails the catalogue lookup nor fails to parse as a call."""
    try:
        connection.execute(f"SELECT {name}({arguments}) FROM probe")
    except duckdb.Error as error:
        message: str = str(error)
        return not (
            any(marker in message for marker in UNKNOWN_FUNCTION_MARKERS)
            or SYNTAX_MARKER in message
        )
    return True


def catalogue_rows(
    *, connection: duckdb.DuckDBPyConnection, listed: list[tuple[str, str, int | None, bool]]
) -> list[tuple[str, str, int | None, bool]]:
    """The listed functions, a fixed-arity window function once per argument count DuckDB binds."""
    rows: list[tuple[str, str, int | None, bool]] = []
    for name, kind, arguments, variadic in listed:
        if kind != WINDOW_KIND or arguments is None or variadic:
            rows.append((name, kind, arguments, variadic))
            continue
        counts: list[int] = window_arities(connection=connection, name=name, listed=arguments)
        rows.extend((name, kind, count, variadic) for count in counts or [arguments])
    return rows


def window_arities(*, connection: duckdb.DuckDBPyConnection, name: str, listed: int) -> list[int]:
    """The argument counts up to `listed` that DuckDB binds for the window function `name`."""
    accepted: list[int] = []
    for count in range(listed + 1):
        arguments: str = ", ".join(["a", *["1"] * (count - 1)]) if count else ""
        try:
            connection.execute(WINDOW_PROBE.format(name=name, arguments=arguments))
        except duckdb.Error:
            continue
        accepted.append(count)
    return accepted
