"""Test outcomes and the injected conversion and execution, so tests need no sqlquack binary."""

from __future__ import annotations

from enum import StrEnum
from pathlib import Path
from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    from scripts.duckshop_measure.models import Conversion


class Outcome(StrEnum):
    """What became of one compiled unit test."""

    MATCHES = "matches"
    DIFFERS = "differs"
    GUARD_RAISED = "guard_raised"
    DUCKDB_ERROR = "duckdb_error"
    REFUSED = "refused"
    NEEDS_PYTHON = "needs_python"


class ModelOutcome(StrEnum):
    """What became of one DuckShop model against Snowflake's recorded rows."""

    MATCHES = "matches"
    MATCHES_AT_MICROSECONDS = "matches_at_microseconds"
    DIFFERS = "differs"
    GUARD_RAISED = "guard_raised"
    DUCKDB_ERROR = "duckdb_error"
    REFUSED = "refused"
    DECLARED_DIVERGENCE = "declared_divergence"
    INPUT_UNRECORDED = "input_unrecorded"
    INPUT_UNLOADABLE = "input_unloadable"
    UNREADABLE = "unreadable"
    NONDETERMINISTIC = "nondeterministic"
    INCREMENTAL = "incremental"
    UNRECORDABLE = "unrecordable"
    SNOWFLAKE_ERROR = "snowflake_error"


class RecordedOutcome(StrEnum):
    """What the recorder captured for one Snowflake relation."""

    ROWS = "rows"
    ERROR = "error"
    UNRECORDABLE = "unrecordable"


class Converter(Protocol):
    """Converts one compiled SQL file."""

    def __call__(self, path: Path) -> Conversion: ...


class Executor(Protocol):
    """Runs converted SQL on DuckDB and returns its rows, or raises DuckDB's error."""

    def __call__(self, sql: str) -> list[tuple[object, ...]]: ...


class Comparer(Protocol):
    """Compares converted models' DuckDB runs with their recordings; outcomes by id."""

    def __call__(
        self, *, manifest: Path, comparisons: list[dict[str, object]]
    ) -> dict[str, dict[str, str]]: ...
