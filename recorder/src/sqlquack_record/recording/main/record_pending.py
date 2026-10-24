"""Record every pending case, batching cases that share session parameters."""

from __future__ import annotations

from collections.abc import Callable, Sequence
from pathlib import Path

from sqlquack_record.recording._helpers.batches import batches, by_session
from sqlquack_record.recording._helpers.results import write_result
from sqlquack_record.recording._helpers.running import run_cases
from sqlquack_record.recording.models import (
    PendingCase,
    RecordingContext,
    RecordSummary,
    StatementResult,
)
from sqlquack_record.recording.types import SessionFactory, WarehouseSession


def record_pending(
    *,
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
    failures: int = 0
    for parameters, group in by_session(unrecorded).items():
        session: WarehouseSession = open_session(parameters)
        try:
            context: RecordingContext = RecordingContext(
                warehouse_version=session.warehouse_version(),
                connector=session.connector(),
                recorded_at=clock(),
            )
            for batch in batches(cases=group, size=batch_size):
                outcomes: list[StatementResult] = run_cases(session=session, batch=batch)
                for case, outcome in zip(batch, outcomes, strict=True):
                    write_result(directory=results, case=case, result=outcome, context=context)
                    recorded += 1
                    failures += outcome.failure is not None
        finally:
            session.close()
    return RecordSummary(recorded=recorded, failures=failures)
