"""Errors raised while building or auditing the Databricks syntax catalogue."""

from __future__ import annotations


class DatabricksCatalogueError(ValueError):
    """A pinned page, grammar file or reviewed row that cannot be read consistently."""
