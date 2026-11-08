"""BigQuery API v2 calls over a transport: retries, jobs, the run datasets and their state."""

from __future__ import annotations

import random
import re
import time
import uuid
from collections.abc import Callable

from scripts.bigquery_receipts._helpers.names import normalise
from scripts.bigquery_receipts._helpers.values import count_of, field, items_of, row_of, rows_of
from scripts.bigquery_receipts.constants import (
    ATTEMPTS,
    BACKOFF_LIMIT_SECONDS,
    BACKOFF_SECONDS,
    BYTE_KEYS,
    DATASET_OPTIONS,
    DONE_STATE,
    DUPLICATE_STATUS,
    HTTP_ERROR_STATUS,
    INTERNAL_REASON,
    INTERNAL_STATUS,
    JOB_PREFIX,
    JOB_TIMEOUT_MS,
    LIST_LIMIT,
    LOCATION,
    MAXIMUM_BYTES_BILLED,
    MISSING,
    NEVER_TOUCHED_PREFIXES,
    NOT_FOUND_STATUS,
    POLL_LIMIT_SECONDS,
    POLL_SECONDS,
    RETRIED_REASONS,
    RETRIED_STATUSES,
    RUN_DATASET_PATTERN,
    RUN_LABEL,
    STOP_REASONS,
    STOP_STATUSES,
    TABLE_EXPIRATION_MS,
    TIMED_CANCEL_SECONDS,
    TIMED_TIMEOUT_MS,
    TOKEN_DATASET,
    TOKEN_DATASET2,
)
from scripts.bigquery_receipts.exceptions import EngineInternalError, RefereeError, StopRun
from scripts.bigquery_receipts.models import EngineMessage, JobResult, RunNames
from scripts.bigquery_receipts.types import Request, Row, Transport


def errors_of(body: Row) -> list[EngineMessage]:
    """The errors of an HTTP error reply or of a finished job, the deciding one first."""
    error: object = body.get("error")
    if isinstance(error, dict):
        listed: list[EngineMessage] = [_engine_message(item) for item in error.get("errors") or []]
        return listed or [_engine_message({"reason": error.get("status"), **error})]
    status: object = body.get("status")
    if not isinstance(status, dict) or not status.get("errorResult"):
        return []
    first: EngineMessage = _engine_message(status["errorResult"])
    rest: list[EngineMessage] = [_engine_message(item) for item in status.get("errors") or []]
    return [first, *(item for item in rest if item != first)]


def _engine_message(item: dict[str, object]) -> EngineMessage:
    return EngineMessage(
        reason=str(item.get("reason") or ""), message=str(item.get("message") or "")
    )


def said(*, errors: list[EngineMessage], names: RunNames) -> str:
    """The errors, normalised, for a referee error."""
    return "; ".join(f"{e.reason}: {normalise(text=e.message, names=names)}" for e in errors)


def _retryable(*, status: int, body: Row) -> bool:
    reasons: set[str] = {error.reason for error in errors_of(body)}
    return status in RETRIED_STATUSES or bool(reasons & RETRIED_REASONS)


def call(
    *,
    transport: Transport,
    request: Request,
    names: RunNames,
    sleep: Callable[[float], None] = time.sleep,
) -> tuple[int, Row]:
    """One API call, retried with backoff on rate limits and backend errors; a stop error stops."""
    method, path, body = request
    status, reply = 0, {}
    for attempt in range(ATTEMPTS):
        status, reply = transport(method=method, path=path, body=body)
        if not _retryable(status=status, body=reply):
            break
        if attempt + 1 < ATTEMPTS:
            delay: float = min(BACKOFF_SECONDS * 2**attempt, BACKOFF_LIMIT_SECONDS)
            sleep(delay + random.uniform(0, delay / 2))
    stop_if_run_error(status=status, body=reply, names=names)
    return status, reply


def stop_if_run_error(*, status: int, body: Row, names: RunNames) -> None:
    """Stop the run on an error that says nothing about the case."""
    errors: list[EngineMessage] = errors_of(body)
    if (
        status in STOP_STATUSES
        or status in RETRIED_STATUSES
        or any(error.reason in STOP_REASONS for error in errors)
    ):
        stated: str = said(errors=errors, names=names)
        internal: bool = status == INTERNAL_STATUS and {e.reason for e in errors} == {
            INTERNAL_REASON
        }
        stop: type[StopRun] = EngineInternalError if internal else StopRun
        raise stop(f"HTTP {status}, the run stops: {stated or 'no error body'}")


def _referenced(query: dict[str, object]) -> list[str]:
    """The datasets a dry run reports the statement reads or targets, as `project.dataset`."""
    items: list[object] = []
    for key in ("referencedTables", "referencedRoutines"):
        items += items_of(query.get(key))
    for key in ("ddlTargetTable", "ddlDestinationTable", "ddlTargetRoutine", "ddlTargetDataset"):
        items.append(query.get(key))
    policy: object = query.get("ddlTargetRowAccessPolicy")
    if isinstance(policy, dict):
        items.append(policy.get("tableReference"))
    return [f"{i.get('projectId')}.{i.get('datasetId')}" for i in items if isinstance(i, dict)]


def _query_statistics(body: Row) -> tuple[dict[str, object], dict[str, object]]:
    statistics: object = body.get("statistics")
    statistics = statistics if isinstance(statistics, dict) else {}
    query: object = statistics.get("query")
    return statistics, query if isinstance(query, dict) else {}


def _job_result(*, body: Row, errors: list[EngineMessage]) -> JobResult:
    _, query = _query_statistics(body)
    return JobResult(
        errors=tuple(errors),
        statement_type=str(query.get("statementType") or ""),
        referenced=tuple(sorted(set(_referenced(query)))),
        raw=body,
    )


def job_record(*, job: JobResult, dry: bool) -> Row:
    """A job as the private record keeps it: whether it was a dry run, and the engine's reply."""
    return {"dry_run": dry, "reply": job.raw}


def bytes_of(jobs: list[Row]) -> dict[str, int]:
    """Bytes processed and billed, summed from the jobs' statistics, dry-run estimates apart."""
    sums: dict[str, int] = dict.fromkeys(BYTE_KEYS, 0)
    for job in jobs:
        statistics, query = _query_statistics(row_of(job["reply"]))
        prefix: str = "dry_run" if job["dry_run"] else "executed"
        sums[f"{prefix}_bytes_processed"] += count_of(statistics.get("totalBytesProcessed"))
        sums[f"{prefix}_bytes_billed"] += count_of(query.get("totalBytesBilled"))
    return sums


def dry_run(*, transport: Transport, sql: str, names: RunNames) -> JobResult:
    """A free dry-run query job of the text, capped in bytes: it parses, resolves, runs nothing."""
    body: Row = {
        "jobReference": {"projectId": names.project, "location": LOCATION},
        "configuration": {
            "dryRun": True,
            "query": {
                "query": sql,
                "useLegacySql": False,
                "maximumBytesBilled": MAXIMUM_BYTES_BILLED,
            },
        },
    }
    request: Request = ("POST", f"/projects/{names.project}/jobs", body)
    _, reply = call(transport=transport, request=request, names=names)
    return _job_result(body=reply, errors=errors_of(reply))


def execute(
    *,
    transport: Transport,
    sql: str,
    names: RunNames,
    sleep: Callable[[float], None] = time.sleep,
    timed: bool = False,
) -> JobResult:
    """A query job of the text, capped in bytes and time; a `timed` one is cancelled if it lasts."""
    job_id: str = JOB_PREFIX + uuid.uuid4().hex
    body: Row = {
        "jobReference": {"projectId": names.project, "jobId": job_id, "location": LOCATION},
        "configuration": {
            "query": {
                "query": sql,
                "useLegacySql": False,
                "maximumBytesBilled": MAXIMUM_BYTES_BILLED,
            },
            "jobTimeoutMs": TIMED_TIMEOUT_MS if timed else JOB_TIMEOUT_MS,
            "labels": {RUN_LABEL: names.dataset},
        },
    }
    path: str = f"/projects/{names.project}/jobs"
    status, reply = call(
        transport=transport, request=("POST", path, body), names=names, sleep=sleep
    )
    if status >= HTTP_ERROR_STATUS and status != DUPLICATE_STATUS:
        return _job_result(body=reply, errors=errors_of(reply))
    waited: float = 0.0
    cancelled: bool = False
    get: Request = ("GET", f"{path}/{job_id}?location={LOCATION}", None)
    cancel: str = f"{path}/{job_id}/cancel?location={LOCATION}"
    while True:
        _, reply = call(transport=transport, request=get, names=names, sleep=sleep)
        state: object = reply.get("status")
        if isinstance(state, dict) and state.get("state") == DONE_STATE:
            return _job_result(body=reply, errors=errors_of(reply))
        if timed and not cancelled and waited >= TIMED_CANCEL_SECONDS:
            transport(method="POST", path=cancel, body=None)
            cancelled = True
        if waited >= POLL_LIMIT_SECONDS:
            transport(method="POST", path=cancel, body=None)
            raise StopRun(f"a job ran past {POLL_LIMIT_SECONDS:.0f} s and was cancelled")
        sleep(POLL_SECONDS)
        waited += POLL_SECONDS


def check_run_dataset(*, dataset: str, names: RunNames) -> None:
    """Refuse any dataset that is not one of this run's two `sqlquack_run_` datasets."""
    if (
        dataset not in (names.dataset, names.dataset2)
        or re.match(RUN_DATASET_PATTERN, dataset) is None
        or dataset.startswith(NEVER_TOUCHED_PREFIXES)
    ):
        raise RefereeError(f"refused to touch a dataset that is not the run's: {dataset}")


def create_dataset(*, transport: Transport, dataset: str, names: RunNames) -> None:
    """Create one run dataset, its tables expiring soon; one that exists already is not adopted."""
    check_run_dataset(dataset=dataset, names=names)
    body: Row = {
        "datasetReference": {"projectId": names.project, "datasetId": dataset},
        "location": LOCATION,
        "labels": {RUN_LABEL: names.dataset},
        "defaultTableExpirationMs": TABLE_EXPIRATION_MS,
    }
    request: Request = ("POST", f"/projects/{names.project}/datasets", body)
    status, reply = call(transport=transport, request=request, names=names)
    if status >= HTTP_ERROR_STATUS:
        stated: str = said(errors=errors_of(reply), names=names)
        raise RefereeError(f"could not create a run dataset (HTTP {status}): {stated}")


def drop_dataset(*, transport: Transport, dataset: str, names: RunNames) -> None:
    """Drop one run dataset with its contents, and check it is gone."""
    check_run_dataset(dataset=dataset, names=names)
    path: str = f"/projects/{names.project}/datasets/{dataset}"
    delete: Request = ("DELETE", f"{path}?deleteContents=true", None)
    status, _ = call(transport=transport, request=delete, names=names)
    gone, _ = call(transport=transport, request=("GET", path, None), names=names)
    if (status >= HTTP_ERROR_STATUS and status != NOT_FOUND_STATUS) or gone != NOT_FOUND_STATUS:
        raise RefereeError(f"a run dataset was not dropped (HTTP {status}, then {gone})")


def _listed(*, transport: Transport, path: str, key: str, names: RunNames) -> list[Row]:
    request: Request = ("GET", f"{path}?maxResults={LIST_LIMIT}", None)
    _, reply = call(transport=transport, request=request, names=names)
    return rows_of(reply.get(key))


def _table_state(*, transport: Transport, path: str, names: RunNames) -> Row:
    """One table's type, row count and fields."""
    _, table = call(transport=transport, request=("GET", path, None), names=names)
    fields: list[Row] = rows_of(row_of(table.get("schema")).get("fields"))
    return {
        "type": table.get("type"),
        "rows": table.get("numRows"),
        "fields": [[field.get("name"), field.get("type"), field.get("mode")] for field in fields],
    }


def dataset_state(*, transport: Transport, dataset: str, names: RunNames) -> Row:
    """A run dataset's options, objects, table schemas and row counts."""
    check_run_dataset(dataset=dataset, names=names)
    path: str = f"/projects/{names.project}/datasets/{dataset}"
    status, reply = call(transport=transport, request=("GET", path, None), names=names)
    if status == NOT_FOUND_STATUS:
        return {"dataset": MISSING}
    tables: dict[str, object] = {}
    for item in _listed(transport=transport, path=f"{path}/tables", key="tables", names=names):
        table_id: str = str(field(value=item, path=("tableReference", "tableId")))
        tables[table_id] = _table_state(
            transport=transport, path=f"{path}/tables/{table_id}", names=names
        )
    listed: list[Row] = _listed(
        transport=transport, path=f"{path}/routines", key="routines", names=names
    )
    routines: list[list[str]] = sorted(
        [
            str(field(value=item, path=("routineReference", "routineId"))),
            str(item.get("routineType")),
        ]
        for item in listed
    )
    models: list[str] = sorted(
        str(field(value=item, path=("modelReference", "modelId")))
        for item in _listed(transport=transport, path=f"{path}/models", key="models", names=names)
    )
    options: Row = {key: reply[key] for key in DATASET_OPTIONS if key in reply}
    labels: object = options.get("labels")
    if isinstance(labels, dict):
        options["labels"] = sorted(labels)
    return {"options": options, "tables": tables, "routines": routines, "models": models}


def run_state(*, transport: Transport, names: RunNames) -> Row:
    """Both run datasets' state, keyed by their tokens so the state names neither."""
    return {
        TOKEN_DATASET: dataset_state(transport=transport, dataset=names.dataset, names=names),
        TOKEN_DATASET2: dataset_state(transport=transport, dataset=names.dataset2, names=names),
    }
