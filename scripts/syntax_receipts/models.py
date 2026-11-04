"""Receipt importer models."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from scripts.syntax_receipts.types import Row


@dataclass(frozen=True)
class Imported:
    """Receipts, their exact inputs, refused rows and run statuses from an import."""

    receipts: list[Row] = field(default_factory=list)
    inputs: dict[str, Row] = field(default_factory=dict)
    refusals: list[Row] = field(default_factory=list)
    runs: list[Row] = field(default_factory=list)


@dataclass(frozen=True)
class CaptureRun:
    """A run directory with its audit events and their hash, and any reconciliation and its hash."""

    path: Path
    events: list[Row]
    audit_sha: str
    reconciliation: Row | None = None
    reconciliation_sha: str | None = None


@dataclass(frozen=True)
class Observation:
    """One recorded statement, bound to its run, packet, exact SQL, setup and session."""

    run_id: str
    audit_sha: str
    raw_bytes: bytes
    case_id: str
    packet_sha: str
    sql_sha: str
    setup: list[str]
    session: Row
    method: str
    accepted: bool
    errno: int | None
    sqlstate: str | None
    message: str
