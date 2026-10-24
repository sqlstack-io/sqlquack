"""Test types for loading pending case files."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class LoadPendingTestCase:
    description: str
    file_name: str
    replace: str
    with_text: str
    expected_references: tuple[str, ...] | None
