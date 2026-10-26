"""Recorder failures."""

from __future__ import annotations


class RecorderError(Exception):
    """A failure that stops recording without writing a result."""


class StatementNotRecordableError(RecorderError):
    """One statement failed with an error that is not a case outcome, such as a compile error."""


class UnrecordableResultError(RecorderError):
    """The warehouse answered, but not in a form whose types the recorder can keep exactly."""
