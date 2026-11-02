"""Dropping statements whose SQL an earlier one already has."""

from __future__ import annotations

from collections.abc import Iterable

from scripts.referee_cases.models import Statement


def unique_statements(statements: Iterable[Statement]) -> list[Statement]:
    """`statements` in order, each SQL text kept once."""
    kept: dict[str, Statement] = {}
    for statement in statements:
        kept.setdefault(statement.sql, statement)
    return list(kept.values())
