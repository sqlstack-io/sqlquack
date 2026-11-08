"""The DuckDB referee's error."""

from __future__ import annotations


class RefereeError(Exception):
    """A run the referee refuses to record: an unmapped error type, or a wrong engine."""
