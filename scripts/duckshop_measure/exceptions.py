"""Errors the DuckShop measures raise."""

from __future__ import annotations


class EnvironmentTableError(ValueError):
    """A referee environment statement that is not `CREATE TEMPORARY TABLE rel__<name> (...)`."""


class UncomparedModelError(LookupError):
    """A recorded model with converted SQL that `sqlquack compare` returned no outcome for."""
