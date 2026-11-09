"""The raises a probe's hooked expression reports, as DuckDB calls the hook."""

from __future__ import annotations


class RaiseLog:
    """Collects each tuple's first raise message, quoted as DuckDB's error quotes it."""

    def __init__(self) -> None:
        self.first: dict[int, str] = {}

    def __call__(self, message: str, index: int) -> int | None:
        """Called by DuckDB for each row reaching a raise; records it and returns NULL."""
        self.first.setdefault(index, f"'{message}'")
        return None
