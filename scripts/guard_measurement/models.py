"""A guarded expression to measure and what measuring it found."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Probe:
    """A guarded Snowflake expression over one relation's columns `x1`..`xn`."""

    divergence: str
    construct: str
    relation: str
    expression: str
    arguments: tuple[tuple[str, str], ...]
    semi_structured: bool | None
    aggregate: bool


@dataclass(frozen=True)
class ConvertedProbe:
    """The DuckDB table macro SQLQuack emitted for a probe."""

    parameters: tuple[str, ...]
    body: str


@dataclass(frozen=True)
class Found:
    """A raising value tuple: its message, its values for reasons only, and its sample rows."""

    message: str
    values: tuple[str | None, ...]
    row_ids: frozenset[int]


@dataclass(frozen=True)
class Evaluation:
    """A probe's raising sample rows, example tuples with messages, and how complete each is."""

    raising_rows: frozenset[int]
    found: tuple[Found, ...]
    exact: bool
    reasons_exact: bool


@dataclass(frozen=True)
class ReasonCount:
    """Rows raising for one reason (scaled from examples when estimated), its label and shapes."""

    origin: str
    reason: str
    divergence: str
    rows: int
    shapes: tuple[str, ...]


@dataclass(frozen=True)
class ProbeResult:
    """One probe's outcome over its relation's sample."""

    probe: Probe
    status: str
    rows: int
    exact: bool
    detail: str
    reasons: tuple[ReasonCount, ...]
    raising_row_ids: frozenset[int]
    label_rows: dict[str, int]
    label_row_ids: dict[str, frozenset[int]]
    reasons_estimated: bool
