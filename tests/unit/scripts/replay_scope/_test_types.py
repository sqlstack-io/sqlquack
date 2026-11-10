"""Test types for the replay scope."""

from __future__ import annotations

from dataclasses import dataclass

from scripts.replay_scope.types import Scope


@dataclass(frozen=True)
class ScopeTestCase:
    description: str
    changed: tuple[str, ...]
    expected_scope: Scope


@dataclass(frozen=True)
class ChecksTestCase:
    description: str
    changed: tuple[str, ...]
    expected_code: bool
    expected_site: bool


@dataclass(frozen=True)
class DigestTestCase:
    description: str
    edited: str
    expected_changed: bool


@dataclass(frozen=True)
class RepositoryTestCase:
    description: str
    expected_unmatched: tuple[str, ...]
    expected_forced_scopes: frozenset[str]
    expected_binary_crates: tuple[bool, bool, bool]


@dataclass(frozen=True)
class RenameTestCase:
    description: str
    source: str
    destination: str
    expected_changed: tuple[str, ...]
