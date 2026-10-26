"""Record every pending case, batching cases that share session parameters."""

from __future__ import annotations

from collections.abc import Callable, Iterator, Sequence
from pathlib import Path

from sqlquack_record.recording._helpers.batches import batches, by_session
from sqlquack_record.recording._helpers.results import write_result
from sqlquack_record.recording._helpers.running import run_cases
from sqlquack_record.recording.exceptions import RecorderError
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
    written: list[str] = []
    failures: int = 0
    refused: list[tuple[str, str]] = []
    try:
        for case, outcome, context in _outcomes(
            cases=unrecorded, open_session=open_session, batch_size=batch_size, clock=clock
        ):
            if outcome.refusal is not None:
                refused.append((case.case_reference, outcome.refusal))
                continue
            write_result(directory=results, case=case, result=outcome, context=context)
            written.append(case.case_reference)
            failures += outcome.failure is not None
    except Exception as error:
        raise RecorderError(f"{error}; {_written_note(written)}") from error
    return RecordSummary(recorded=len(written), failures=failures, refused=tuple(refused))


def _outcomes(
    *,
    cases: Sequence[PendingCase],
    open_session: SessionFactory,
    batch_size: int,
    clock: Callable[[], str],
) -> Iterator[tuple[PendingCase, StatementResult, RecordingContext]]:
    for parameters, group in by_session(cases).items():
        session: WarehouseSession = open_session(parameters)
        try:
            context: RecordingContext = RecordingContext(
                warehouse_version=session.warehouse_version(),
                connector=session.connector(),
                recorded_at=clock(),
                session_parameters=tuple(sorted(session.session_parameters().items())),
            )
            for batch in batches(cases=group, size=batch_size):
                outcomes: list[StatementResult] = run_cases(session=session, batch=batch)
                for case, outcome in zip(batch, outcomes, strict=True):
                    yield case, outcome, context
        finally:
            session.close()


def _written_note(written: Sequence[str]) -> str:
    if not written:
        return "no result was written by this run"
    return (
        f"{len(written)} results were written by this run before it stopped, the last for "
        f"{written[-1]}; they are complete and stay, and rerunning records only the rest"
    )
