"""Record every pending referee case, one session per environment, and the catalogue."""

from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.referee._helpers.results import write_catalogue
from sqlquack_record.referee._helpers.running import record_environment
from sqlquack_record.referee.constants import FUNCTIONS_DIRECTORY, RESULTS_DIRECTORY
from sqlquack_record.referee.models import (
    RefereeCase,
    RefereeContext,
    RefereeEnvironment,
    RefereePending,
    RefereeSummary,
)
from sqlquack_record.referee.types import RefereeSession, RefereeSessionFactory


def record_referee(
    *,
    pending: RefereePending,
    directory: Path,
    open_session: RefereeSessionFactory,
    clock: Callable[[], str],
) -> RefereeSummary:
    """Compile each unrecorded case once and store its outcome; never overwrite one."""
    results: Path = directory / RESULTS_DIRECTORY
    results.mkdir(parents=True, exist_ok=True)
    environments: dict[str, RefereeEnvironment] = {
        environment.environment_sha256: environment for environment in pending.environments
    }
    groups: dict[str, list[RefereeCase]] = {}
    for case in pending.cases:
        if not (results / f"{case.case_hash}.json").exists():
            groups.setdefault(case.environment_sha256, []).append(case)
    recorded: int = 0
    accepted: int = 0
    refused: list[tuple[str, str]] = []
    for digest, cases in groups.items():
        environment: RefereeEnvironment | None = environments.get(digest)
        if environment is None:
            refused.extend((case.case_reference, "no pending environment") for case in cases)
            continue
        try:
            environment_accepted, environment_refused = record_environment(
                environment=environment,
                cases=cases,
                directory=results,
                open_session=open_session,
                clock=clock,
            )
        except Exception as error:
            raise RecorderError(
                f"{error}; {recorded} referee outcomes were written before it stopped and stay"
            ) from error
        refused.extend(environment_refused)
        recorded += len(cases) - len(environment_refused)
        accepted += environment_accepted
    catalogue: str | None = None
    if pending.catalogue:
        catalogue = _catalogue(directory=directory, open_session=open_session, clock=clock)
    return RefereeSummary(
        recorded=recorded, accepted=accepted, refused=tuple(refused), catalogue=catalogue
    )


def _catalogue(
    *, directory: Path, open_session: RefereeSessionFactory, clock: Callable[[], str]
) -> str | None:
    session: RefereeSession = open_session(())
    try:
        context: RefereeContext = RefereeContext(
            warehouse_version=session.warehouse_version(),
            connector=session.connector(),
            recorded_at=clock(),
            session_parameters=(),
            identity=(),
        )
        return write_catalogue(
            directory=directory / FUNCTIONS_DIRECTORY,
            rows=session.builtin_functions(),
            context=context,
        )
    finally:
        session.close()
