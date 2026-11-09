"""The committed timing budget."""

from __future__ import annotations

from typing import TypedDict


class Timing(TypedDict):
    """One workload's generated SQL size and its time over the reference's, as last recorded."""

    sql_bytes: int
    seconds: float
    reference_seconds: float
    ratio: float


class BudgetFile(TypedDict):
    """Every workload's recorded timing, at `rows` rows."""

    format: str
    rows: int
    workloads: dict[str, Timing]
