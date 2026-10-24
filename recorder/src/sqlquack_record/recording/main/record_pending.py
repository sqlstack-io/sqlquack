"""Record every pending case, batching cases that share session parameters."""

from __future__ import annotations

from collections.abc import Callable, Sequence
from pathlib import Path

from sqlquack_record.errors import RecorderError
from sqlquack_record.models import PendingCase, RecordingContext, RecordSummary, StatementResult
from sqlquack_record.recording._helpers.batches import batches, by_session
from sqlquack_record.results.main.write_result import write_result
from sqlquack_record.types import SessionFactory, WarehouseSession


def record_pending(
    cases: Sequence[PendingCase],
    results: Path,
    open_session: SessionFactory,
    batch_size: int,
    clock: Callable[[], str],
) -> RecordSummary:
    """Run each unrecorded case once and store its result; never overwrite a stored result."""
    results.mkdir(parents=True, exist_ok=True)
    unrecorded: list[PendingCase] = [
        case for case in cases if not (results / f"{case.case_hash}.json").exists()
    ]
    recorded: int = 0
    errors: int = 0
    for parameters, group in by_session(unrecorded).items():
        session: WarehouseSession = open_session(parameters)
        try:
            context: RecordingContext = RecordingContext(
                warehouse_version=session.warehouse_version(),
                connector=session.connector(),
                recorded_at=clock(),
            )
            for batch in batches(group, batch_size):
                outcomes: list[StatementResult] = _run(session, batch)
                for case, outcome in zip(batch, outcomes, strict=True):
                    write_result(results, case, outcome, context)
                    recorded += 1
                    errors += outcome.error is not None
        finally:
            session.close()
    return RecordSummary(recorded=recorded, errors=errors)


def _run(session: WarehouseSession, batch: Sequence[PendingCase]) -> list[StatementResult]:
    statements: list[str] = [case.source_sql for case in batch]
    batched: list[StatementResult] | None = (
        session.run_batch(statements) if len(statements) > 1 else None
    )
    if batched is not None:
        if len(batched) != len(statements):
            raise RecorderError(
                f"the warehouse returned {len(batched)} results for {len(statements)} statements"
            )
        return batched
    return [session.run_one(statement) for statement in statements]
