"""Test types for the website examples."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ExampleCheckTestCase:
    description: str
    page: str
    homepage: str
    printed: str
    expected_return_code: int
    expected_fragment: str
