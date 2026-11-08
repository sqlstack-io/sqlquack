"""What the referee observed for one case."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Observation:
    """One case's phase, the error that decided it and whether the parse-only pass refused it."""

    phase: str
    basis: str
    engine_code: str | None
    parse_only_code: str | None
    message: str | None
    statements: int
