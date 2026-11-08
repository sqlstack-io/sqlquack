"""Loading DuckDB's extensions and probing the names its parser handles itself."""

from __future__ import annotations

import duckdb

from scripts.duckdb_catalog.constants import (
    PARSER_KEYWORDS_QUERY,
    PROBE_ARGUMENTS,
    PROBE_TABLE,
    SYNTAX_MARKER,
    TRANSFORMER_REWRITTEN_NAMES,
    UNKNOWN_FUNCTION_MARKERS,
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
    connection.execute(PROBE_TABLE)
    keywords: list[str] = [row[0] for row in connection.execute(PARSER_KEYWORDS_QUERY).fetchall()]
    special: list[str] = []
    for name in sorted({*keywords, *TRANSFORMER_REWRITTEN_NAMES} - known):
        if any(
            reaches_past_lookup(connection=connection, name=name, arguments=arguments)
            for arguments in PROBE_ARGUMENTS
        ):
            special.append(name)
    return special


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
