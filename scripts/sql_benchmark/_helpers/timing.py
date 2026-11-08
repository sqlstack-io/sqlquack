"""Converting a workload and timing it and its reference on DuckDB."""

from __future__ import annotations

import json
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any

import duckdb

from scripts.sql_benchmark.constants import (
    MAX_REPEATS,
    MIN_REPEATS,
    ROWS_PLACEHOLDER,
    SCHEMA,
    TIMED_SECONDS,
)
from scripts.sql_benchmark.models import Workload
from scripts.sql_benchmark.types import Timing


def timed(*, workload: Workload, sqlquack: Path, policy: Path, rows: int) -> Timing:
    """The workload converted, then it and its reference timed over fresh rows."""
    report: dict[str, Any] = converted(sql=workload.snowflake, sqlquack=sqlquack, policy=policy)
    connection: duckdb.DuckDBPyConnection = duckdb.connect()
    connection.execute("SET TimeZone = 'UTC'")
    connection.execute(workload.rows.replace(ROWS_PLACEHOLDER, str(rows * workload.row_factor)))
    for statement in report["macros"]:
        connection.execute(statement)
    seconds: float = fastest(connection=connection, sql=report["sql"])
    reference: float = fastest(connection=connection, sql=workload.reference)
    return {
        "sql_bytes": len(report["sql"].encode()),
        "seconds": round(seconds, 3),
        "reference_seconds": round(reference, 3),
        "ratio": round(seconds / reference, 1),
    }


def converted(*, sql: str, sqlquack: Path, policy: Path) -> dict[str, Any]:
    """`sqlquack convert --json`'s report for `sql`, which must convert."""
    with tempfile.TemporaryDirectory() as directory:
        schema: Path = Path(directory) / "schema.json"
        schema.write_text(json.dumps(SCHEMA))
        completed: subprocess.CompletedProcess[str] = subprocess.run(
            [str(sqlquack), "convert", "--json", "--compact", "--utc-session"]
            + ["--schema", str(schema), "--policy", str(policy)],
            input=sql,
            capture_output=True,
            text=True,
            check=False,
        )
    report: dict[str, Any] = json.loads(completed.stdout)
    if not report["converted"]:
        raise SystemExit(f"sql benchmark: SQLQuack refused {sql!r}: {completed.stderr}")
    return report


def fastest(*, connection: duckdb.DuckDBPyConnection, sql: str) -> float:
    """The fastest run of `sql`, rerun until TIMED_SECONDS pass, every row materialised."""
    times: list[float] = []
    while len(times) < MIN_REPEATS or (sum(times) < TIMED_SECONDS and len(times) < MAX_REPEATS):
        started: float = time.perf_counter()
        connection.execute(f"CREATE OR REPLACE TEMP TABLE benchmark_result AS {sql}")
        times.append(time.perf_counter() - started)
    return min(times)
