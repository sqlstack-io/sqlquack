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
class SchemaFileTestCase:
    description: str
    schema: str
    expected_schema: str
