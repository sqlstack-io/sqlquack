"""The BigQuery referee's errors."""

from __future__ import annotations


class RefereeError(Exception):
    """A run the referee refuses to record: an unmapped error, a leaked name or changed state."""


class StopRun(RefereeError):
    """An error that says nothing about the case (permission, quota, billing, backend)."""


class EngineInternalError(StopRun):
    """An internalError that every retry of one request repeats; held when few cases raise it."""
