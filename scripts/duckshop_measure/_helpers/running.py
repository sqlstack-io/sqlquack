"""Conversion by the sqlquack binary and execution in a fresh DuckDB with replay's session."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import duckdb

from scripts.duckshop_measure.constants import REPLAY_SESSION
from scripts.duckshop_measure.models import Conversion
from scripts.duckshop_measure.types import Converter


def refusal_names(refusals: object) -> tuple[str, ...]:
    """Each refusal as `category: construct`, sorted and distinct."""
    listed: list[dict[str, str]] = refusals if isinstance(refusals, list) else []
    return tuple(sorted({f"{refusal['category']}: {refusal['construct']}" for refusal in listed}))


def binary_converter(*, binary: Path, library: Path, policy: Path) -> Converter:
    """A converter running `<binary> convert --json` under `policy`, libduckdb from `library`."""
    environment: dict[str, str] = {**os.environ, "LD_LIBRARY_PATH": str(library)}

    def convert(path: Path) -> Conversion:
        """The conversion of `path`; a crash or unreadable output is a refusal naming it."""
        completed: subprocess.CompletedProcess[str] = subprocess.run(
            (str(binary), "convert", str(path), "--json", "--policy", str(policy)),
            capture_output=True,
            text=True,
            check=False,
            env=environment,
        )
        try:
            report: dict[str, object] = json.loads(completed.stdout)
        except json.JSONDecodeError:
            return Conversion(
                converted=False,
                sql="",
                refusals=(f"no report: exit {completed.returncode}",),
                python_functions=False,
            )
        return Conversion(
            converted=report.get("converted") is True,
            sql=str(report.get("sql") or ""),
            refusals=refusal_names(report.get("refusals")),
            python_functions=bool(report.get("python_functions")),
        )

    return convert


def duckdb_rows(sql: str) -> list[tuple[object, ...]]:
    """Every row `sql` returns in a new in-memory DuckDB with replay's session settings."""
    connection: duckdb.DuckDBPyConnection = duckdb.connect()
    try:
        connection.execute(REPLAY_SESSION)
        return connection.execute(sql).fetchall()
    finally:
        connection.close()
