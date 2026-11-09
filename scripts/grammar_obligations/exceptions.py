"""Errors raised while deriving syntax obligations."""

from __future__ import annotations


class ObligationError(Exception):
    """A limit or derived obligation that cannot be written consistently."""
