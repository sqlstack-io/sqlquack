"""Record every pending case, batching cases that share session parameters."""

from __future__ import annotations

from collections.abc import Callable, Sequence
from pathlib import Path

from sqlquack_record.recording._helpers.outcomes import outcomes
from sqlquack_record.recording._helpers.results import write_result
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.models import (
    PendingCase,
    RecordSummary,
)
from sqlquack_record.recording.types import SessionFactory


def record_pending(
    *,
    cases: Sequence[PendingCase],
    results: Path,
    open_session: SessionFactory,
    batch_size: int,
    clock: Callable[[], str],
) -> RecordSummary:
    """Run each unrecorded case once, after every case compiles; never overwrite a result."""
    results.mkdir(parents=True, exist_ok=True)
    unrecorded: list[PendingCase] = [
        case for case in cases if not (results / f"{case.case_hash}.json").exists()
    ]
    written: list[str] = []
    failures: int = 0
    refused: list[tuple[str, str]] = []
    try:
        for case, outcome, context in outcomes(
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


def _written_note(written: Sequence[str]) -> str:
    if not written:
        return "no result was written by this run"
    return (
        f"{len(written)} results were written by this run before it stopped, the last for "
        f"{written[-1]}; they are complete and stay, and rerunning records only the rest"
    )
