"""Referee cases as the pending script reads them from `referee/`."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class EnvironmentFile:
    """An environment file: session parameters and the setup statements creating its tables."""

    name: str
    session_parameters: dict[str, bool | int | str]
    setup_sql: tuple[str, ...]
    environment_sha256: str


@dataclass(frozen=True)
class RefereeCase:
    """One statement of a referee file, with the hash naming it and its environment."""

    case_hash: str
    case_reference: str
    environment: EnvironmentFile
    statement: str
    syntax_only: bool
    kind: str
    mutation: str | None
    base: str | None


@dataclass(frozen=True)
class ProjectStatement:
    """A compiled model as a referee statement, and the relations it reads."""

    name: str
    sql: str
    tag: str
    relations: frozenset[str]


@dataclass(frozen=True)
class ProjectCorpus:
    """Compiled models usable as referee statements, the declared relations, and the rest."""

    statements: tuple[ProjectStatement, ...]
    tables: dict[str, tuple[dict[str, Any], ...]]
    skipped: dict[str, int]


@dataclass(frozen=True)
class ProjectImport:
    """Where a compiled project is, what to call it, and which of its models to keep."""

    name: str
    compiled: Path
    dag: Path
    origin: str
    split_tag: str | None
    every: int


@dataclass(frozen=True)
class ParseVerdict:
    """SQLQuack's parse of one statement: whether it parses, and the parse error if not."""

    parses: bool
    reason: str


@dataclass(frozen=True)
class Comparison:
    """One referee case with Snowflake's recorded outcome and SQLQuack's parse."""

    case: RefereeCase
    outcome: dict[str, Any] | None
    verdict: ParseVerdict
    category: str
