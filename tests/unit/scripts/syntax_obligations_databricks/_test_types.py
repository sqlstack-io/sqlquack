"""Test types for the Databricks obligation build's tombstone guard."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class TombstoneTestCase:
    description: str
    limits: str
    retired: str
    expected_error: str
    expected_present: bool
    expected_unchanged: bool


@dataclass(frozen=True)
class CheckTestCase:
    description: str
    relative: str
    old: str
    new: str
    expected_code: int
    expected_stale: int


@dataclass(frozen=True)
class StrayTestCase:
    description: str
    relative: str
    expected_code: int


@dataclass(frozen=True)
class ReservedWordTestCase:
    description: str
    runtime_only: bool
    expected_keyword_expect: str


@dataclass(frozen=True)
class BareFormTestCase:
    description: str
    form: str
    expected_sql: str


@dataclass(frozen=True)
class MissingOpenTestCase:
    description: str
    expected_unbalanced: tuple[str, ...]


@dataclass(frozen=True)
class PreviewSettlementTestCase:
    description: str
    case_id: str
    expected_settlement: tuple[str, str] | None


@dataclass(frozen=True)
class SettlementBuildTestCase:
    description: str
    family: str
    phase: str
    expected_derived_expect: str
    expected_written: tuple[str, str]


@dataclass(frozen=True)
class KindSettlementTestCase:
    description: str
    kinds: set[str]
    expect: str
    settlement: tuple[str, str]
    expected_settlement: tuple[str, str] | None
