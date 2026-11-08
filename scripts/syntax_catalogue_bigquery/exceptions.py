"""Errors raised while fetching, building or auditing the BigQuery syntax catalogue."""

from __future__ import annotations


class BigQueryCatalogueError(ValueError):
    """A pinned page, grammar file or reviewed input that cannot be read consistently."""
