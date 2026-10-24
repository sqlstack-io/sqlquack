"""Evidence guard models."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ChangedPath:
    """One path a change touches, with Git's status letter."""

    status: str
    path: str
