"""Errors raised while building or auditing the SQLite syntax catalogue."""

from __future__ import annotations


class SqliteCatalogueError(ValueError):
    """A pinned archive, page, grammar rule or hand row that cannot be read consistently."""
