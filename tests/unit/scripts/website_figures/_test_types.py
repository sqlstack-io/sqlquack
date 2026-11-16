"""Test types for the website figures."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class WriteTestCase:
    description: str
    day: str
    expected_figures: dict[str, str]


@dataclass(frozen=True)
class CheckTestCase:
    description: str
    edits: dict[str, Any]
    expected_return_code: int
    expected_fragment: str
