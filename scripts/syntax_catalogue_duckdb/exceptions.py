"""Errors raised while building or auditing the DuckDB syntax catalogue."""

from __future__ import annotations


class DuckDbCatalogueError(ValueError):
    """A pinned page, grammar rule or source file that cannot be read consistently."""
