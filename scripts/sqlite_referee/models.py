"""What the referee prepares, steps and observes."""

from __future__ import annotations

import ctypes
from dataclasses import dataclass


@dataclass(frozen=True)
class Prepared:
    """One sqlite3_prepare_v2 call: the statement or the error, and where the statement ended."""

    statement: ctypes.c_void_p | None
    end: int
    code: int
    name: str
    message: str


@dataclass(frozen=True)
class StepResult:
    """A statement stepped to completion: SQLITE_OK, or the error that stopped it."""

    code: int
    name: str
    message: str


@dataclass(frozen=True)
class Observation:
    """One case's phase, the error that decided it and whether the parse-only pass refused it."""

    phase: str
    basis: str
    engine_code: str | None
    parse_only_code: str | None
    message: str | None
    statements: int
