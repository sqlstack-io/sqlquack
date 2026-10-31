"""Running pending cases session by session, once all of them compile."""

from __future__ import annotations

from collections.abc import Callable, Iterator, Sequence

from sqlquack_record.recording._helpers.batches import batches, by_session
from sqlquack_record.recording._helpers.precheck import compile_failures
from sqlquack_record.recording._helpers.running import run_cases
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.models import PendingCase, RecordingContext, StatementResult
from sqlquack_record.recording.types import SessionFactory, SessionParameters, WarehouseSession


def outcomes(
    *,
    cases: Sequence[PendingCase],
    open_session: SessionFactory,
    batch_size: int,
    clock: Callable[[], str],
) -> Iterator[tuple[PendingCase, StatementResult, RecordingContext]]:
    """Each case's outcome and recording facts, once every case compiles or declares it won't."""
    groups: dict[SessionParameters, list[PendingCase]] = by_session(cases)
    sessions: dict[SessionParameters, WarehouseSession] = {}
    try:
        for parameters in groups:
            sessions[parameters] = open_session(parameters)
        failures: list[str] = compile_failures(groups=groups, sessions=sessions)
        if failures:
            raise RecorderError(
                f"{len(failures)} cases do not compile on Snowflake, so none was run: "
                + "; ".join(failures)
                + "; fix them, or declare `compile_error = true` on a case recording the refusal"
            )
        for parameters, group in groups.items():
            session: WarehouseSession = sessions.pop(parameters)
            try:
                yield from _group_outcomes(
                    session=session, group=group, batch_size=batch_size, clock=clock
                )
            finally:
                session.close()
    finally:
        for session in sessions.values():
            session.close()


def _group_outcomes(
    *,
    session: WarehouseSession,
    group: Sequence[PendingCase],
    batch_size: int,
    clock: Callable[[], str],
) -> Iterator[tuple[PendingCase, StatementResult, RecordingContext]]:
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
