"""A block a website page shows as an example."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ShownBlock:
    """A block's page and line, the example it shows, how to read it, and its text."""

    page: str
    line: int
    example: str
    spec: str
    text: str
