"""The basics corpus's statements."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Basic:
    """One basic statement: its shape, and its SQL with an alias on each item, or without."""

    shape: str
    sql: str
    aliased: bool
