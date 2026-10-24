"""Pending cases, statement results and recorded metadata."""

from __future__ import annotations

from dataclasses import dataclass

import pyarrow as pa

SessionValue = bool | int | str


@dataclass(frozen=True)
class PendingCase:
    """One case to record, as `sqlquack pending` writes it."""

    case_hash: str
    case_reference: str
    source_dialect: str
    session_parameters: tuple[tuple[str, SessionValue], ...]
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
class RecordedError:
    """A warehouse error recorded as a case outcome."""

    errno: int | None
    sqlstate: str | None
    message: str


@dataclass(frozen=True)
class StatementResult:
    """What the warehouse returned for one case: rows as Arrow, or an error."""

    query_id: str | None
    columns: tuple[ColumnDescription, ...]
    table: pa.Table | None
    error: RecordedError | None


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
    errors: int
