"""Failures of the sqlquack API."""

from __future__ import annotations


class SqlQuackError(ValueError):
    """The SQL could not be checked or converted at all, where the command line exits 2."""
