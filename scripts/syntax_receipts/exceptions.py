"""Errors the receipt importer raises."""

from __future__ import annotations


class RunRefused(Exception):
    """The whole run cannot be imported."""


class RowRefused(ValueError):
    """One recorded row cannot become a receipt."""
