"""Errors the transformations raise when the source does not have the shape they expect."""

from __future__ import annotations


class TransformError(Exception):
    """The source cannot be transformed as planned; the message names where and why."""
