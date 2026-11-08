"""The SQLite referee's error."""

from __future__ import annotations


class RefereeError(Exception):
    """A run the referee refuses to record: an unmapped error, or a wrong engine."""
