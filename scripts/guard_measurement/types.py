"""Enumerations and callable shapes the measurement uses."""

from __future__ import annotations

from enum import StrEnum
from typing import Protocol

from scripts.guard_measurement.models import ConvertedProbe


class ProbeStatus(StrEnum):
    """What became of a probe."""

    MEASURED = "measured"
    NO_SAMPLE = "no sample"
    SKIPPED = "skipped"
    REFUSED = "refused"
    FAILED = "failed"


class Origin(StrEnum):
    """Where a raise comes from: a guard, a Snowflake error SQLQuack raises, or DuckDB itself."""

    DIVERGENCE = "divergence"
    SNOWFLAKE_ERROR = "snowflake_error"
    DUCKDB_ERROR = "duckdb_error"


class Converter(Protocol):
    """Converts a probe body over sample columns into a DuckDB table macro, or gives the refusal."""

    def __call__(
        self, *, body: str, columns: tuple[tuple[str, str], ...]
    ) -> ConvertedProbe | str: ...
