"""Test types for recording referee cases."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class RefereeTestCase:
    description: str
    setup_sql: tuple[str, ...]
    statements: tuple[str, ...]
    syntax_only: bool
    preexisting: tuple[str, ...]
    expected_outcomes: tuple[str, ...]
    expected_refused: int
    expected_log: tuple[str, ...]


@dataclass(frozen=True)
class MessageTestCase:
    description: str
    statement: str
    syntax_only: bool
    expected_message: str | None


@dataclass(frozen=True)
class GuardTestCase:
    description: str
    statement: str
    expected_refused: bool


@dataclass(frozen=True)
class PendingFileTestCase:
    description: str
    folder: str
    key: str
    value: object
    expected_error: str


@dataclass(frozen=True)
class CatalogueTestCase:
    description: str
    runs: int
    expected_names: list[list[str]]
    expected_last_written: bool


@dataclass(frozen=True)
class ParityTestCase:
    description: str
    selection: str
    expected_environments: int
