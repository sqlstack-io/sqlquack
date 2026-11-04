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
    values: bool = True
    setup_sql: tuple[str, ...] = ()
    pinned_parameters: tuple[tuple[str, bool | int | str], ...] = ()
    compile_error: bool = False

    def recording_parameters(self) -> tuple[tuple[str, bool | int | str], ...]:
        """The pinned and case parameters the recording session sets, sorted by name."""
        return tuple(
            sorted({**dict(self.pinned_parameters), **dict(self.session_parameters)}.items())
        )


@dataclass(frozen=True)
class ColumnDescription:
    """One result column as the connector describes it; µs values may be proven whole."""

    name: str
    type_code: int
    type_name: str
    precision: int | None
    scale: int | None
    internal_size: int | None
    nullable: bool | None
    sub_microsecond_checked: bool = False


@dataclass(frozen=True)
class WarehouseFailure:
    """A warehouse SQL error recorded as a case outcome, its message without the query ID."""

    errno: int | None
    sqlstate: str | None
    message: str | None = None


@dataclass(frozen=True)
class StatementResult:
    """What the warehouse returned for one case: rows as Arrow, or a SQL error."""

    query_id: str | None
    columns: tuple[ColumnDescription, ...]
    table: pa.Table | None
    failure: WarehouseFailure | None
    refusal: str | None = None


@dataclass(frozen=True)
class RecordingContext:
    """Facts about the recording session written into every result."""

    warehouse_version: str
    connector: str
    recorded_at: str
    session_parameters: tuple[tuple[str, str], ...] = ()


@dataclass(frozen=True)
class RecordSummary:
    """What a recording run did."""

    recorded: int
    failures: int
    refused: tuple[tuple[str, str], ...] = ()


@dataclass(frozen=True)
class ScratchFunctions:
    """A case's functions renamed for its run, and the statements dropping them afterwards."""

    setup_sql: tuple[str, ...]
    source_sql: str
    drop_sql: tuple[str, ...]
