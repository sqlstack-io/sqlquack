"""Test types for the referee generators."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class DeterminismTestCase:
    description: str
    runs: int
    expected_duplicate_names: int


@dataclass(frozen=True)
class CheckTestCase:
    description: str
    tamper: Callable[[Path], None]
    expected_return_code: int
    expected_fragment: str


@dataclass(frozen=True)
class MutationTestCase:
    description: str
    base: str
    expected_mutation: str
    expected_sql: str
