"""Errors publishing the evidence raises."""

from __future__ import annotations


class SupersessionError(ValueError):
    """A receipt that supersedes a line no receipt of the same root publishes."""
