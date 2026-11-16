"""Test types for the syntax obligation generator."""

from __future__ import annotations

from dataclasses import dataclass

from scripts.grammar_obligations.types import Row


@dataclass(frozen=True)
class ShortenedTargetTestCase:
    description: str
    grammar: str
    expected_last_tokens: frozenset[str]


@dataclass(frozen=True)
class SetupBindingTestCase:
    description: str
    receipt_sql_sha256: str
    report_input_sql_sha256: str
    expected_setup: tuple[str, ...]


@dataclass(frozen=True)
class DisputedFormMarkTestCase:
    description: str
    sql: str
    expected_mark: str
    expected_expect: str


@dataclass(frozen=True)
class DisprovedExpectationTestCase:
    description: str
    sql: str
    settlement: str
    phase: str
    expected_expect: str
    expected_mark: str | None


@dataclass(frozen=True)
class ProbeSettlementTestCase:
    description: str
    receipts: tuple[Row, ...]
    expected_expect: str
    expected_mark: str | None


@dataclass(frozen=True)
class PackedRefusalTestCase:
    description: str
    refused_sql: str
    phase: str
    expected_option_sql: tuple[str, ...]


@dataclass(frozen=True)
class RenderedCaseTestCase:
    description: str
    catalogue: str
    unpacked: frozenset[str]
    sql: str
    expected_expect: str
    expected_note: str
