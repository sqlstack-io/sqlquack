"""Running one batch of cases."""

from __future__ import annotations

import secrets
from collections.abc import Sequence
from dataclasses import replace

from sqlquack_record.recording._helpers.functions import scratch_functions
from sqlquack_record.recording.constants import SCRATCH_NONCE_BYTES
from sqlquack_record.recording.exceptions import RecorderError, StatementNotRecordableError
from sqlquack_record.recording.models import PendingCase, ScratchFunctions, StatementResult
from sqlquack_record.recording.types import WarehouseSession


def run_cases(*, session: WarehouseSession, batch: Sequence[PendingCase]) -> list[StatementResult]:
    """One result per case, in order; cases creating functions run alone in a scratch schema."""
    plain: list[PendingCase] = [case for case in batch if not case.setup_sql]
    answered: dict[str, StatementResult] = dict(
        zip(
            (case.case_hash for case in plain),
            _run_plain(session=session, batch=plain),
            strict=True,
        )
    )
    return [
        answered[case.case_hash]
        if not case.setup_sql
        else _run_with_functions(session=session, case=case)
        for case in batch
    ]


def _run_plain(*, session: WarehouseSession, batch: Sequence[PendingCase]) -> list[StatementResult]:
    """One request for the batch, or one per case so a statement error names its case."""
    statements: list[str] = [case.source_sql for case in batch]
    try:
        batched: list[StatementResult] | None = (
            session.run_batch(statements) if len(statements) > 1 else None
        )
    except StatementNotRecordableError:
        batched = None
    if batched is None:
        return [_run_one(session=session, case=case) for case in batch]
    if len(batched) != len(statements):
        raise RecorderError(
            f"the warehouse returned {len(batched)} results for {len(statements)} statements"
        )
    return batched


def _run_with_functions(*, session: WarehouseSession, case: PendingCase) -> StatementResult:
    """The case's functions created under unique names in the session's schema, then dropped."""
    if session.current_schema() is None:
        raise RecorderError(
            f"case {case.case_reference} creates functions: "
            "set SNOWFLAKE_SCHEMA to a scratch schema"
        )
    scratch: ScratchFunctions = scratch_functions(
        setup_sql=case.setup_sql,
        source_sql=case.source_sql,
        case_hash=case.case_hash,
        run_nonce=secrets.token_hex(SCRATCH_NONCE_BYTES),
    )
    try:
        for statement in scratch.setup_sql:
            try:
                session.execute(statement)
            except RecorderError as error:
                raise RecorderError(
                    f"case {case.case_reference} (pending file {case.case_hash}.json): {error}"
                ) from error
        return _run_one(session=session, case=replace(case, source_sql=scratch.source_sql))
    finally:
        for statement in scratch.drop_sql:
            session.execute(statement)


def _run_one(*, session: WarehouseSession, case: PendingCase) -> StatementResult:
    try:
        return session.run_one(case.source_sql, expected_error=case.expected_error)
    except RecorderError as error:
        raise RecorderError(
            f"case {case.case_reference} (pending file {case.case_hash}.json) failed: {error}"
        ) from error
