"""Test types for the evidence guard."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class GuardTestCase:
    description: str
    change: str
    expected_return_code: int
    expected_fragment: str
