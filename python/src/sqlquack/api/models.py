"""Diagnostics and results, as the command line reports them."""

from __future__ import annotations

import os
from collections.abc import Mapping
from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class Span:
    """Where a diagnostic points: character offsets, then one-based line and column positions."""

    start: int
    end: int
    line: int
    column: int
    end_line: int
    end_column: int


@dataclass(frozen=True)
class Diagnostic:
    """One coded problem; `span` is absent only for problems outside the SQL, like sessions."""

    code: str
    name: str
    alias: str | None
    severity: str
    message: str
    span: Span | None
    label: str
    notes: tuple[str, ...]
    help: str | None


@dataclass(frozen=True)
class CheckResult:
    """What `check` found, with `rendered` as the command line prints it."""

    ok: bool
    diagnostics: tuple[Diagnostic, ...]
    rendered: str


@dataclass(frozen=True)
class Conversion:
    """DuckDB SQL or the refusing diagnostics; `report` is exactly `convert --json`'s."""

    converted: bool
    sql: str
    macros: tuple[str, ...]
    profile: str
    diagnostics: tuple[Diagnostic, ...]
    divergences: Mapping[str, tuple[str, ...]]
    assumptions: tuple[str, ...]
    divergence_failures: Mapping[str, str]
    notes: tuple[str, ...]
    rendered: str
    report: Mapping[str, Any]


@dataclass(frozen=True)
class ConvertOptions:
    """The `sqlquack convert` flags, one field each, and `schema`, declarations held in memory."""

    project: str | os.PathLike[str] | None = None
    schema: Mapping[str, Any] | None = None
    config: str | os.PathLike[str] | None = None
    no_config: bool = False
    session: Mapping[str, str] = field(default_factory=dict)
    strict: bool = False
    python_runtime: bool = False
    overrides: str | os.PathLike[str] | None = None
    verified_overrides: Mapping[str, str] = field(default_factory=dict)
    compact: bool = False
