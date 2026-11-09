"""Errors that stop a Databricks referee run."""

from __future__ import annotations


class RefereeError(RuntimeError):
    """A case or reply the referee cannot judge; the run writes no receipt."""


class TokenRejected(RefereeError):
    """The workspace rejected the bearer token, so the request did nothing."""


class ReplyLost(RefereeError):
    """A statement's reply never arrived, so it may have run and must not be sent again."""
