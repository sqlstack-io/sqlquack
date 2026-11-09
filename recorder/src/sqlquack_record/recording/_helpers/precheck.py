"""Compiling every pending case before anything is recorded."""

from __future__ import annotations

from collections.abc import Mapping, Sequence

from sqlquack_record.recording.constants import COMPILE_FAILURE_SQLSTATE_CLASSES
from sqlquack_record.recording.models import PendingCase, WarehouseFailure
from sqlquack_record.recording.types import SessionParameters, WarehouseSession


def compile_failures(
    *,
    groups: Mapping[SessionParameters, Sequence[PendingCase]],
    sessions: Mapping[SessionParameters, WarehouseSession],
) -> list[str]:
    """Each case Snowflake will not compile, but those declaring it or creating functions."""
    failures: list[str] = []
    for parameters, group in groups.items():
        for case in group:
            if case.setup_sql or case.compile_error:
                continue
            failure: WarehouseFailure | None = sessions[parameters].compile_failure(case.source_sql)
            sqlstate: str = "" if failure is None else str(failure.sqlstate or "")
            if failure is not None and sqlstate[:2] in COMPILE_FAILURE_SQLSTATE_CLASSES:
                failures.append(
                    f"{case.case_reference} (pending file {case.case_hash}.json): Snowflake "
                    f"error {failure.errno} (SQLSTATE {sqlstate}): {failure.message or ''}"
                )
    return failures
