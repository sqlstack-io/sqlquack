"""Test types for the Snowflake obligation build's --check and its family selection."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class CheckTestCase:
    description: str
    family: str
    relative: str
    old: str
    new: str
    expected_code: int
    expected_stale: tuple[str, ...]


@dataclass(frozen=True)
class MissingFileTestCase:
    description: str
    family: str
    relative: str
    expected_code: int


@dataclass(frozen=True)
class SelectionTestCase:
    description: str
    relative: str
    old: str
    new: str
    expected_families: tuple[str, ...]


@dataclass(frozen=True)
class SkipTestCase:
    description: str
    families: tuple[str, ...]
    expected_lines: tuple[str, ...]


@dataclass(frozen=True)
class ClosureTestCase:
    description: str
    module: str
    selection_only: tuple[str, ...]
    expected_outside: tuple[str, ...]


@dataclass(frozen=True)
class BranchTestCase:
    description: str
    branch_edit: tuple[str, str, str]
    main_edit: tuple[str, str, str]
    expected_families: tuple[str, ...]


@dataclass(frozen=True)
class StrayElisionTestCase:
    description: str
    arguments: tuple[str, ...]
    expected_code: int
    expected_text: str


@dataclass(frozen=True)
class OperandClassTestCase:
    description: str
    target: str
    expected_notes: frozenset[str]
    family: str = "transaction_session"


@dataclass(frozen=True)
class DerivedLimitTestCase:
    description: str
    target: str
    sql: str
    expected_count: int


@dataclass(frozen=True)
class ListItemTestCase:
    description: str
    kind: str
    expected_sql: tuple[str, ...]


@dataclass(frozen=True)
class DocumentedRefusalTestCase:
    description: str
    grammar: str
    target: str
    sql: str
    expected_note: str
