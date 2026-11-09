"""Errors the evidence pin check raises."""

from __future__ import annotations


class PinFileError(ValueError):
    """An evidence.toml that does not parse, or whose commit or content hash is malformed."""


class SnapshotError(RuntimeError):
    """The pinned snapshot scripts/evidence.sh could not fetch, extract or verify."""
