"""Evidence pin models."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class EvidencePin:
    """What evidence.toml pins: one commit of the evidence repository and the hash of its files."""

    repository: str
    commit: str
    content_sha256: str


@dataclass(frozen=True)
class ChangedPath:
    """One path a change between two evidence commits touches, with Git's status letter."""

    status: str
    path: str
