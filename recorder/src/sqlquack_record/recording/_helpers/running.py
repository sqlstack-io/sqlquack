"""Running one batch of cases."""

from __future__ import annotations

from collections.abc import Sequence

from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.models import PendingCase, StatementResult
from sqlquack_record.recording.types import WarehouseSession


def run_cases(*, session: WarehouseSession, batch: Sequence[PendingCase]) -> list[StatementResult]:
    """One result per case: one request for the batch, or one per case after a SQL error."""
    statements: list[str] = [case.source_sql for case in batch]
    batched: list[StatementResult] | None = (
        session.run_batch(statements) if len(statements) > 1 else None
    )
    if batched is None:
        return [session.run_one(statement) for statement in statements]
    if len(batched) != len(statements):
        raise RecorderError(
            f"the warehouse returned {len(batched)} results for {len(statements)} statements"
        )
    return batched
