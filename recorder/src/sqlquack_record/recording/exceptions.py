"""Recorder failures."""

from __future__ import annotations


class RecorderError(Exception):
    """A failure that stops recording without writing a result."""
