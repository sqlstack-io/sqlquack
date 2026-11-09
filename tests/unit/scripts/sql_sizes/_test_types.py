"""Test types for the generated-SQL size ratchet."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class GrowthTestCase:
    description: str
    base: dict[str, int]
    measured: dict[str, int]
    allowances: str
    expected_code: int
    expected_fragment: str


@dataclass(frozen=True)
class BaselineTestCase:
    description: str
    committed: dict[str, int]
    measured: dict[str, int]
    expected_codes: tuple[int, int, int]
    expected_fragment: str
