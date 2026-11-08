"""Test types for the BigQuery obligation build's tombstone guard."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class TombstoneTestCase:
    description: str
    limits: str
    retired: str
    expected_error: str
    expected_present: bool


@dataclass(frozen=True)
class GateTestCase:
    description: str
    expect: str
    stage: str
    parsed: bool
    message: str
    expected_disagreements: int
    expected_unmapped: list[str]
    receipt_phase: str = ""


@dataclass(frozen=True)
class TokensTestCase:
    description: str
    sql: str
    expected_sql: str


@dataclass(frozen=True)
class WithdrawalTestCase:
    description: str
    sql: str
    parsed: bool
    expected_error: str


@dataclass(frozen=True)
class SettleTestCase:
    description: str
    parsed: bool
    settle: frozenset[str]
    expected_expect: str
    message: str = "m"


@dataclass(frozen=True)
class ShapeTestCase:
    description: str
    parsed: bool
    message: str
    expected_error: str


@dataclass(frozen=True)
class ClassTestCase:
    description: str
    form: str
    grammar: str
    placeholder: str
    expected_class: str
