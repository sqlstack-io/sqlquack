"""Test types for reading compiled function headers."""

from __future__ import annotations

from dataclasses import dataclass

from scripts.census_functions.models import Signature


@dataclass(frozen=True)
class HeaderTestCase:
    description: str
    header: str
    expected_signature: Signature | None


@dataclass(frozen=True)
class ProjectFilesTestCase:
    description: str
    compiled: tuple[tuple[str, str], ...]
    expected_files: dict[str, str]
