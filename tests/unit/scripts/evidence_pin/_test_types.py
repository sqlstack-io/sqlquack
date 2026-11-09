"""Test types for the evidence pin."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import NamedTuple


class EvidenceRepositories(NamedTuple):
    """A bare evidence remote, a working copy pushing to it, a SQLQuack checkout and a clone."""

    remote: Path
    author: Path
    root: Path
    clone: Path


class ScriptOutcome(NamedTuple):
    """How the last run of scripts/evidence.sh in a scenario ended, and what else held after it."""

    return_code: int
    stdout: str
    stderr: str
    facts: dict[str, bool]


type Change = Callable[[EvidenceRepositories], str]
type Preparation = Callable[[EvidenceRepositories], None]
type Scenario = Callable[[EvidenceRepositories], ScriptOutcome]


@dataclass(frozen=True)
class PinMoveTestCase:
    description: str
    change: Change
    expected_return_code: int
    expected_fragment: str


@dataclass(frozen=True)
class PinCheckTestCase:
    description: str
    prepare: Preparation
    expected_return_code: int
    expected_fragment: str


@dataclass(frozen=True)
class ScriptTestCase:
    description: str
    scenario: Scenario
    expected_return_code: int
    expected_fragment: str


@dataclass(frozen=True)
class ContentHashTestCase:
    description: str
    files: dict[str, bytes]
    links: dict[str, str]
    expected_files_hashed: int
