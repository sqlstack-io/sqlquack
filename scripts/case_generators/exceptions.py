"""Errors the case generators raise."""

from __future__ import annotations


class StrictJsonError(ValueError):
    """JSON text a strict parser rejects: a repeated key, NaN or Infinity."""
