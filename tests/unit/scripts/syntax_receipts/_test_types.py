"""Test types for the syntax receipt importer."""

from __future__ import annotations

import json
from collections.abc import Callable
from dataclasses import dataclass, field

from scripts.syntax_receipts.types import Row


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
class CaptureDriverTestCase:
    description: str
    start: Row
    expected_status: str
    expected_capture_methods: list[str]


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
    expected_setup: list[str]
    expected_refusals: list[str]


@dataclass(frozen=True)
class SupersedeTestCase:
    description: str
    rows: list[dict[str, object]]
    packet: list[dict[str, object]]
    published_phase: str
    expected_new_receipts: int
    expected_withdrawn: list[tuple[str, str]]


@dataclass(frozen=True)
class ReconciledRunTestCase:
    description: str
    edit: Callable[[Row], Row]
    expected_receipts: list[tuple[str, str]]
    expected_refusals: list[tuple[str, str]]
    start: Row = field(default_factory=lambda: {"run_database": "SQLQUACK_DDL_ABC123", "setup": []})
    audit_events: int = 2
    partial_line: bytes = b""
    render: Callable[[Row], str] = json.dumps
    file_name: str = "reconciliation.json"


@dataclass(frozen=True)
class QueryHistoryReceiptTestCase:
    description: str
    edit: Callable[[Row], Row]
    expected_receipt: Row
