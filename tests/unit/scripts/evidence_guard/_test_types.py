"""Test types for the evidence guard."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

type Change = Callable[[Path, str], None]


@dataclass(frozen=True)
class GuardTestCase:
    description: str
    change: Change
    expected_return_code: int
    expected_fragment: str
