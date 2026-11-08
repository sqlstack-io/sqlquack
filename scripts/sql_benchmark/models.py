"""One benchmarked query."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Workload:
    """Snowflake SQL, its rows (`row_factor` times the benchmark's count) and a DuckDB reference."""

    name: str
    rows: str
    snowflake: str
    reference: str
    row_factor: int = 1
