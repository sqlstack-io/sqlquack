"""Test types for the clutter guard."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

type Change = Callable[[Path], None]


@dataclass(frozen=True)
class GuardTestCase:
    description: str
    change: Change
    expected_return_code: int
    expected_fragment: str


@dataclass(frozen=True)
class JsonGuardTestCase:
    description: str
    change: Change
    expected_counts: dict[str, int]
    expected_findings: list[dict[str, object]]
