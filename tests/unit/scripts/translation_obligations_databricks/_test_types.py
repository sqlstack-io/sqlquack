"""Test types for the Databricks translation obligation build."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class BuildTestCase:
    description: str
    edit: str
    expected_code: int
    expected_cited: tuple[tuple[str, ...], ...]
