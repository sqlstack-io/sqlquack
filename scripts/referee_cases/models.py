"""Synthetic tables, generated statements and the referee files they are written to."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class TableColumn:
    """One column of an empty synthetic table."""

    name: str
    data_type: str
    nullable: bool = True


@dataclass(frozen=True)
class Table:
    """An empty synthetic table, created as a temporary table before its cases compile."""

    name: str
    columns: tuple[TableColumn, ...]


@dataclass(frozen=True)
class Environment:
    """Named tables and session parameters that a group of statements compiles against."""

    name: str
    tables: tuple[Table, ...]
    session_parameters: tuple[tuple[str, bool | int | str], ...]


@dataclass(frozen=True)
class Statement:
    """One referee statement: valid as written, a sweep variant, or a mutant of a valid one."""

    name: str
    sql: str
    kind: str
    mutation: str | None = None
    base: str | None = None


@dataclass(frozen=True)
class RefereeFile:
    """One file of statements sharing an environment, written below `referee/`."""

    path: str
    origin: str
    environment: str
    syntax_only: bool
    statements: tuple[Statement, ...]


@dataclass(frozen=True)
class Function:
    """A Snowflake built-in: its canonical arguments and how many it takes at most."""

    name: str
    arguments: tuple[str, ...]
    maximum: int | None
    minimum: int
    shape: str = "scalar"
