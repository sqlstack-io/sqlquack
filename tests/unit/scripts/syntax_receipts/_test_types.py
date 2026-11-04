"""Test types for the syntax receipt importer."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class ClassifyTestCase:
    description: str
    errno: int | None
    message: str
    expected_classification: tuple[str, str] | None
    sqlstate: str | None = None


@dataclass(frozen=True)
class BindingTestCase:
    description: str
    rows: list[dict[str, object]]
    packet: list[dict[str, object]]
    expected_receipts: list[tuple[str, str]]
    expected_refusals: list[tuple[str, str]]
    expected_sql_sha256: str
    expected_raw_record_sha256: str


@dataclass(frozen=True)
class RefusedRunTestCase:
    description: str
    rows: list[dict[str, object]]
    packet: list[dict[str, object]]
    raw_hash: str
    expected_status: str
    expected_reason_fragment: str


@dataclass(frozen=True)
class OverwriteTestCase:
    description: str
    rows: list[dict[str, object]]
    packet: list[dict[str, object]]
    expected_error: type[Exception]


@dataclass(frozen=True)
class RepublishTestCase:
    description: str
    rows: list[dict[str, object]]
    packet: list[dict[str, object]]
    fields_added_later: tuple[str, ...]
    expected_new_receipts: str


@dataclass(frozen=True)
class SetupRunTestCase:
    description: str
    rows: list[dict[str, object]]
    packet: list[dict[str, object]]
    start: dict[str, object]
    expected_receipts: list[tuple[str, str, str]]
    expected_setup_tail: list[str]
    expected_refusals: list[str]


@dataclass(frozen=True)
class SupersedeTestCase:
    description: str
    rows: list[dict[str, object]]
    packet: list[dict[str, object]]
    published_phase: str
    expected_new_receipts: int
    expected_withdrawn: list[tuple[str, str]]
