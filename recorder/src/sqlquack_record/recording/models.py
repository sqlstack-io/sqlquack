"""Pending cases, statement results and recording facts."""

from __future__ import annotations

from dataclasses import dataclass

import pyarrow as pa


@dataclass(frozen=True)
class PendingCase:
    """One case to record, as `sqlquack pending` writes it."""

    case_hash: str
    case_reference: str
    source_dialect: str
    session_parameters: tuple[tuple[str, bool | int | str], ...]
    source_sql: str
    duckdb_version: str


@dataclass(frozen=True)
class ColumnDescription:
    """One result column as the connector describes it."""

    name: str
    type_code: int
    type_name: str
    precision: int | None
    scale: int | None
    internal_size: int | None
    nullable: bool | None


@dataclass(frozen=True)
class WarehouseFailure:
    """A warehouse SQL error recorded as a case outcome; messages can name account objects."""

    errno: int | None
    sqlstate: str | None


@dataclass(frozen=True)
class StatementResult:
    """What the warehouse returned for one case: rows as Arrow, or a SQL error."""

    query_id: str | None
    columns: tuple[ColumnDescription, ...]
    table: pa.Table | None
    failure: WarehouseFailure | None


@dataclass(frozen=True)
class RecordingContext:
    """Facts about the recording session written into every result."""

    warehouse_version: str
    connector: str
    recorded_at: str


@dataclass(frozen=True)
class RecordSummary:
    """What a recording run did."""

    recorded: int
    failures: int
