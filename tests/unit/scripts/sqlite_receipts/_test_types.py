"""Test types for the SQLite referee's phase mapping."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class PhaseTestCase:
    description: str
    code: int
    message: str
    expected_code: str
    expected_phase: str
    statement: str = "SELECT 1"
    alone: str | None = None


@dataclass(frozen=True)
class UnreviewedPhaseTestCase:
    description: str
    code: int
    message: str
    expected_error: str
    statement: str = "SELECT 1"
    alone: str | None = None
