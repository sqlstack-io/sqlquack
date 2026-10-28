"""Test types for the case generators."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class CheckTestCase:
    description: str
    tamper: Callable[[Path], None]
    expected_return_code: int
    expected_fragment: str


@dataclass(frozen=True)
class FileTestCase:
    description: str
    name: str
    expected_origin_prefix: str


@dataclass(frozen=True)
class HazardTestCase:
    description: str
    case_id: str
    expected_rows: tuple[tuple[str, ...], ...]


@dataclass(frozen=True)
class FindingTestCase:
    description: str
    table: str
    expected_entry: str


@dataclass(frozen=True)
class ExpectationTestCase:
    description: str
    case_id: str
    expected_divergence: str | None
