"""Test types for the Databricks referee's phase table."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class PhaseTestCase:
    description: str
    message: str
    sqlstate: str
    expected_phase: str | None
    expected_parse_only: str | None
