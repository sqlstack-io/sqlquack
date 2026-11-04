"""Syntax catalogue errors."""

from __future__ import annotations


class CatalogueError(ValueError):
    """A hand form is malformed, its page changed, or its page is not in the cache."""
