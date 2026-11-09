"""Clutter guard models."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, order=True)
class Finding:
    """One piece of clutter at a path and line, 0 when the path itself is the clutter."""

    path: str
    line: int
    clutter_class: str
    snippet: str
