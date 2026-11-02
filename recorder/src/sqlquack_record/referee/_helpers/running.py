"""Compiling the cases of one environment in one session."""

from __future__ import annotations

import sys
from collections.abc import Callable, Sequence
from pathlib import Path

from sqlquack_record.recording.exceptions import InternalCompileError, RecorderError
from sqlquack_record.recording.models import ColumnDescription, WarehouseFailure
from sqlquack_record.referee._helpers.guards import setup_refusal, statement_refusal
from sqlquack_record.referee._helpers.results import write_outcome
from sqlquack_record.referee.constants import PROGRESS_EVERY
from sqlquack_record.referee.models import (
    RefereeCase,
    RefereeContext,
    RefereeEnvironment,
    RefereeOutcome,
)
from sqlquack_record.referee.types import RefereeSession, RefereeSessionFactory


def record_environment(
    *,
    environment: RefereeEnvironment,
    cases: Sequence[RefereeCase],
    directory: Path,
    open_session: RefereeSessionFactory,
    clock: Callable[[], str],
) -> tuple[int, list[tuple[str, str]]]:
    """Accepted count and refusals; temporary tables vanish with the session, never dropped."""
    refusal: str | None = next(
        (reason for reason in map(setup_refusal, environment.setup_sql) if reason is not None),
        None,
    )
    if refusal is not None:
        return 0, [(case.case_reference, refusal) for case in cases]
    refused: list[tuple[str, str]] = []
    runnable: list[RefereeCase] = []
    for case in cases:
        reason: str | None = statement_refusal(case.statement)
        if reason is None:
            runnable.append(case)
        else:
            refused.append((case.case_reference, reason))
    if not runnable:
        return 0, refused
    session: RefereeSession = open_session(environment.session_parameters)
    try:
        try:
            for statement in environment.setup_sql:
                session.execute(statement)
        except RecorderError as error:
            return 0, refused + [(case.case_reference, str(error)) for case in runnable]
        context: RefereeContext = RefereeContext(
            warehouse_version=session.warehouse_version(),
            connector=session.connector(),
            recorded_at=clock(),
            session_parameters=_effective(session=session, environment=environment),
            identity=session.identity(),
        )
        accepted: int = 0
        for index, case in enumerate(runnable, start=1):
            try:
                outcome: RefereeOutcome = _outcome(session=session, case=case)
            except InternalCompileError as error:
                refused.append((case.case_reference, str(error)))
                continue
            write_outcome(directory=directory, case=case, outcome=outcome, context=context)
            accepted += outcome.error is None
            if index % PROGRESS_EVERY == 0:
                print(f"sqlquack-record: referee {index}/{len(runnable)} compiled", file=sys.stderr)
    finally:
        session.close()
    return accepted, refused


def _outcome(*, session: RefereeSession, case: RefereeCase) -> RefereeOutcome:
    failure: WarehouseFailure | None = session.compile_failure(case.statement)
    if failure is not None or case.syntax_only:
        return RefereeOutcome(error=failure, columns=None, describe_error=None)
    described: tuple[ColumnDescription, ...] | WarehouseFailure = session.describe(case.statement)
    if isinstance(described, WarehouseFailure):
        return RefereeOutcome(error=None, columns=None, describe_error=described)
    return RefereeOutcome(error=None, columns=described, describe_error=None)


def _effective(
    *, session: RefereeSession, environment: RefereeEnvironment
) -> tuple[tuple[str, str], ...]:
    reported: dict[str, str] = session.session_parameters()
    return tuple(
        (name, reported.get(name, "")) for name, _ in sorted(environment.session_parameters)
    )
