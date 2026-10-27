"""Test types for the fensu transformations."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class TransformTestCase:
    description: str
    source: str
    expected_source: str
