"""One unit test's conversion and how its converted SQL fared on DuckDB."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from scripts.duckshop_measure.types import ModelOutcome


@dataclass(frozen=True)
class Conversion:
    """What `sqlquack convert --json` made of one file."""

    converted: bool
    sql: str
    refusals: tuple[str, ...]
    python_functions: bool


@dataclass(frozen=True)
class UnitTestOutcome:
    """One test file's outcome, with the refused constructs, failing steps or error behind it."""

    test: str
    outcome: str
    details: tuple[str, ...] = field(default_factory=tuple)


@dataclass(frozen=True)
class ModelResult:
    """One model's outcome, with its refused constructs, difference, error or reason."""

    model: str
    target: str
    outcome: ModelOutcome
    details: tuple[str, ...] = field(default_factory=tuple)


@dataclass(frozen=True)
class SnowflakeInputs:
    """Where the recordings, compiled models, census report, conversions and scratch are."""

    results: Path
    compiled: Path
    census: Path
    converted: Path
    lenient_converted: Path | None
    guards: Path | None
    value_divergences: frozenset[str]
    work: Path
