"""Referee cases, the environments they compile in, and what a referee run recorded."""

from __future__ import annotations

from dataclasses import dataclass

from sqlquack_record.recording.models import ColumnDescription, WarehouseFailure
from sqlquack_record.recording.types import SessionParameters


@dataclass(frozen=True)
class RefereeEnvironment:
    """Empty tables and session parameters a referee case compiles against."""

    environment_sha256: str
    session_parameters: SessionParameters
    setup_sql: tuple[str, ...]


@dataclass(frozen=True)
class RefereeCase:
    """One statement whose compile verdict and result columns Snowflake decides."""

    case_hash: str
    case_reference: str
    source_dialect: str
    environment_sha256: str
    statement: str
    syntax_only: bool


@dataclass(frozen=True)
class RefereePending:
    """Every pending referee case and environment, and whether the catalogue is requested."""

    cases: tuple[RefereeCase, ...]
    environments: tuple[RefereeEnvironment, ...]
    catalogue: bool


@dataclass(frozen=True)
class RefereeOutcome:
    """The `EXPLAIN` verdict and, for an accepted statement, its described result columns."""

    error: WarehouseFailure | None
    columns: tuple[ColumnDescription, ...] | None
    describe_error: WarehouseFailure | None


@dataclass(frozen=True)
class RefereeContext:
    """Facts about one referee session written into each of its outcomes."""

    warehouse_version: str
    connector: str
    recorded_at: str
    session_parameters: tuple[tuple[str, str], ...]
    identity: tuple[tuple[str, str], ...]


@dataclass(frozen=True)
class RefereeSummary:
    """What a referee run did."""

    recorded: int
    accepted: int
    refused: tuple[tuple[str, str], ...]
    catalogue: str | None
