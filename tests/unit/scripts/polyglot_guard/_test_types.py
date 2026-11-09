"""Test types for the polyglot guard."""

from __future__ import annotations

from dataclasses import dataclass

type Edge = tuple[str, str, str | None]


@dataclass(frozen=True)
class GuardTestCase:
    description: str
    members: tuple[str, ...]
    edges: tuple[Edge, ...]
    expected_return_code: int
    expected_fragment: str
