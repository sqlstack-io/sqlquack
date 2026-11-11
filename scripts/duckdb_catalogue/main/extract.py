"""Catalogue extract entry point: DuckDB's functions and extensions, and a manifest."""

from __future__ import annotations

import argparse
import csv
import io
from pathlib import Path
from typing import Any

import duckdb

from scripts.duckdb_catalogue._helpers.engine import (
    catalogue_rows,
    load_extensions,
    special_forms,
    syntax_forms,
)
from scripts.duckdb_catalogue.constants import BUILT_IN_FUNCTIONS_QUERY, DATA_DIR, PROBE_TABLE


def run_catalog_extract(argv: list[str] | None = None) -> int:
    """Write functions.csv and manifest.toml from DuckDB with every extension it can load."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--extension-directory", required=True, type=Path)
    options: argparse.Namespace = parser.parse_args(argv)
    connection: duckdb.DuckDBPyConnection = duckdb.connect(
        config={"extension_directory": str(options.extension_directory)}
    )
    loaded, unloadable = load_extensions(connection)
    connection.execute(PROBE_TABLE)
    rows: list[tuple[str, str, int | None, bool]] = catalogue_rows(
        connection=connection, listed=connection.execute(BUILT_IN_FUNCTIONS_QUERY).fetchall()
    )
    known: set[str] = {row[0] for row in rows}
    special: list[str] = special_forms(connection=connection, known=known)
    syntax: list[str] = syntax_forms(connection)
    output: io.StringIO = io.StringIO()
    writer: Any = csv.writer(output, lineterminator="\n")
    writer.writerow(["name", "kind", "arguments", "variadic"])
    for name, kind, arguments, variadic in rows:
        writer.writerow([name, kind, "" if arguments is None else arguments, str(variadic).lower()])
    for name in sorted({*special, *syntax} - known):
        writer.writerow([name, "special", "", "false"])
    (DATA_DIR / "functions.csv").write_text(output.getvalue())
    (DATA_DIR / "manifest.toml").write_text(
        'engine = "duckdb"\n'
        f'engine_version = "{duckdb.__version__}"\n'
        f"extensions = {sorted(loaded)!r}\n".replace("'", '"')
        + f"unloadable_extensions = {sorted(unloadable)!r}\n".replace("'", '"')
        + 'method = "duckdb_functions() with operators, parser probes of duckdb_keywords() '
        "and probes of the syntax the parser emits as calls; "
        'written by scripts/extract_duckdb_catalogue.py"\n'
    )
    return 0
