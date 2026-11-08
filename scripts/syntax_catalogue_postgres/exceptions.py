"""Errors raised while building or auditing the PostgreSQL syntax catalogue."""

from __future__ import annotations


class PgCatalogueError(ValueError):
    """A pinned page, grammar rule or source file that cannot be read consistently."""
