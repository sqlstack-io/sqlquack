"""Judge one case on BigQuery: a dry run, then execution where the policy needs it."""

from __future__ import annotations

import re
import time
from collections.abc import Callable
from dataclasses import replace

from scripts.bigquery_referee._helpers.jobs import dry_run, execute, job_record
from scripts.bigquery_referee._helpers.names import normalise, positions, substitute
from scripts.bigquery_referee._helpers.refusals import syntax_classes
from scripts.bigquery_referee.constants import (
    ACCEPTED,
    BASIS_CASE_DATASETS,
    BASIS_DRY,
    BASIS_DRY_ANY,
    BASIS_DRY_CLEAN,
    BASIS_EXECUTE,
    BASIS_RAN,
    BASIS_RAN_CASE_DATASETS,
    BASIS_RAN_ROLLED_BACK,
    BASIS_ROLLED_BACK,
    BASIS_TIMED,
    CASE_DATASET_CLASSES,
    CASE_DATASET_FAMILIES,
    CASE_DATASET_GUARD,
    CASE_DATASET_HELD,
    CASE_DATASET_PATTERN,
    CASE_DATASET_TARGET,
    DRY_RUN_CLASSES,
    EXECUTE_CLASSES,
    EXECUTED_FAMILIES,
    EXECUTION_FAMILIES,
    EXECUTION_GUARD,
    EXPORT_BUCKET,
    EXPORT_HELD,
    EXPORT_PATTERN,
    EXPORT_URI,
    FAMILY_HELD,
    GUARD_HELD,
    HELD,
    INTERNAL_HELD,
    MODE_CASE_DATASETS,
    MODE_DRY_ONLY,
    MODE_EXECUTE,
    MODE_HELD,
    MODE_ROLLED_BACK,
    MODE_TIMED,
    OUTSIDE_HELD,
    ROLLED_BACK_FAMILY,
    ROLLED_BACK_GUARD,
    ROLLED_BACK_PATTERN,
    ROLLED_BACK_PREFIX,
    ROLLED_BACK_SUFFIX,
    SYNTAX_ERROR,
    TIMED_CLASSES,
    TIMED_PATTERN,
    UNMAPPED_HEAD,
)
from scripts.bigquery_referee.exceptions import EngineInternalError
from scripts.bigquery_referee.models import (
    EngineMessage,
    ErrorClass,
    JobResult,
    Observation,
    RunNames,
)
from scripts.bigquery_referee.types import Row, Transport


def execution_mode(*, case_id: str, sql: str) -> tuple[str, str]:
    """How a case with a clean dry run is judged, and why it is held when it is."""
    family: str = case_id.split(".")[1]
    if family not in EXECUTION_FAMILIES:
        return MODE_DRY_ONLY, ""
    for pattern, reason in HELD:
        if re.search(pattern, sql, re.I):
            return MODE_HELD, reason
    if re.search(EXPORT_PATTERN, sql, re.I):
        uris: list[str] = re.findall(EXPORT_URI, sql)
        if not uris or not all(uri.startswith(EXPORT_BUCKET) for uri in uris):
            return MODE_HELD, EXPORT_HELD
    if family == ROLLED_BACK_FAMILY and re.match(ROLLED_BACK_PATTERN, sql, re.I):
        if re.search(ROLLED_BACK_GUARD, sql, re.I):
            return MODE_HELD, GUARD_HELD
        return MODE_ROLLED_BACK, ""
    if family in EXECUTED_FAMILIES:
        if re.search(EXECUTION_GUARD, sql, re.I):
            return MODE_HELD, GUARD_HELD
        if re.search(TIMED_PATTERN, sql, re.I):
            return MODE_TIMED, ""
        return MODE_EXECUTE, ""
    if family in CASE_DATASET_FAMILIES or re.search(CASE_DATASET_PATTERN, sql, re.I):
        if re.search(CASE_DATASET_GUARD, sql, re.I) or re.search(CASE_DATASET_TARGET, sql, re.I):
            return MODE_HELD, CASE_DATASET_HELD
        return MODE_CASE_DATASETS, ""
    return MODE_HELD, FAMILY_HELD[family]


def dry_run_classes() -> tuple[ErrorClass, ...]:
    """The dry run's classes: the pinned parser's refusals first, then the semantic classes."""
    return (*syntax_classes(), *DRY_RUN_CLASSES)


def error_class(
    *, error: EngineMessage, classes: tuple[ErrorClass, ...], said: Callable[[str], str]
) -> ErrorClass | None:
    """The reviewed class of an error: its reason and a pattern on its normalised message."""
    text: str = said(error.message)
    return next(
        (item for item in classes if item.reason == error.reason and re.search(item.pattern, text)),
        None,
    )


def message_text(*, errors: list[EngineMessage], said: Callable[[str], str]) -> str:
    """The engine's errors, normalised, one `reason: message` per error."""
    return "\n".join(f"{error.reason}: {said(error.message)}" for error in errors)


def decided(
    *,
    errors: list[EngineMessage],
    classes: tuple[ErrorClass, ...],
    basis: str,
    parse_only: bool,
    said: Callable[[str], str],
) -> Observation:
    """The phase of the first syntax error, else of the first error; unmapped ones are listed."""
    found: list[tuple[EngineMessage, ErrorClass | None]] = [
        (error, error_class(error=error, classes=classes, said=said)) for error in errors
    ]
    message: str = message_text(errors=errors, said=said)
    unmapped: tuple[str, ...] = tuple(
        dict.fromkeys(
            f"{error.reason}: {said(error.message)[:UNMAPPED_HEAD]}"
            for error, kind in found
            if kind is None
        )
    )
    if unmapped:
        return Observation(
            phase="",
            basis="",
            engine_code=None,
            parse_only_code=None,
            message=message,
            unmapped=unmapped,
        )
    known: list[tuple[EngineMessage, ErrorClass]] = [(e, k) for e, k in found if k is not None]
    syntax: list[tuple[EngineMessage, ErrorClass]] = [
        p for p in known if p[1].phase == SYNTAX_ERROR
    ]
    deciding, kind = syntax[0] if syntax else known[0]
    first: ErrorClass = known[0][1]
    stated: str = (
        basis.format(reason=deciding.reason, code=kind.code)
        if kind is first or not parse_only
        else BASIS_DRY_ANY.format(reason=deciding.reason, code=kind.code, first=first.code)
    )
    return Observation(
        phase=kind.phase,
        basis=stated,
        engine_code=kind.code,
        parse_only_code=kind.code if parse_only else None,
        message=message,
    )


def _normaliser(*, sql: str, names: RunNames, line_offset: int) -> Callable[[str], str]:
    """Normalise engine text of a submission of `sql`, positions in the committed text."""
    position: Callable[[str], str] = positions(sql=sql, names=names, line_offset=line_offset)
    return lambda text: normalise(text=text, names=names, position=position)


def observe_dry(*, case: Row, transport: Transport, names: RunNames) -> Observation:
    """The dry run's verdict, or the execution the policy asks for (`mode`) or a hold."""
    sql: str = str(case["sql"])
    try:
        dry: JobResult = dry_run(
            transport=transport, sql=substitute(sql=sql, names=names), names=names
        )
    except EngineInternalError:
        return Observation(
            phase="",
            basis="",
            engine_code=None,
            parse_only_code=None,
            message=None,
            held=INTERNAL_HELD,
        )
    jobs: tuple[Row, ...] = (job_record(job=dry, dry=True),)
    if dry.errors:
        judged: Observation = decided(
            errors=list(dry.errors),
            classes=dry_run_classes(),
            basis=BASIS_DRY,
            parse_only=True,
            said=_normaliser(sql=sql, names=names, line_offset=0),
        )
        return replace(judged, jobs=jobs)
    mode, reason = execution_mode(case_id=str(case["case_id"]), sql=sql)
    run_datasets: set[str] = {f"{names.project}.{d}" for d in (names.dataset, names.dataset2)}
    if (
        mode in (MODE_EXECUTE, MODE_ROLLED_BACK, MODE_CASE_DATASETS, MODE_TIMED)
        and not set(dry.referenced) <= run_datasets
    ):
        mode, reason = MODE_HELD, OUTSIDE_HELD
    if mode == MODE_DRY_ONLY:
        return Observation(
            phase=ACCEPTED,
            basis=BASIS_DRY_CLEAN,
            engine_code=None,
            parse_only_code=None,
            message=None,
            jobs=jobs,
        )
    return Observation(
        phase="",
        basis="",
        engine_code=None,
        parse_only_code=None,
        message=None,
        held=reason if mode == MODE_HELD else "",
        mode=mode,
        jobs=jobs,
    )


def rolled_back(sql: str) -> str:
    """The statement inside a transaction that is rolled back, so it changes nothing."""
    return ROLLED_BACK_PREFIX + sql.strip().rstrip(";").rstrip() + ROLLED_BACK_SUFFIX


def observe_execution(
    *,
    case: Row,
    pending: Observation,
    transport: Transport,
    names: RunNames,
    sleep: Callable[[float], None] = time.sleep,
) -> Observation:
    """Run a case the dry run accepted and the policy executes, in the datasets of `names`."""
    sql: str = str(case["sql"])
    wrapped: bool = pending.mode == MODE_ROLLED_BACK
    text: str = substitute(sql=sql, names=names)
    submitted: str = rolled_back(text) if wrapped else text
    ran: JobResult = execute(
        transport=transport,
        sql=submitted,
        names=names,
        sleep=sleep,
        timed=pending.mode == MODE_TIMED,
    )
    jobs: tuple[Row, ...] = (*pending.jobs, job_record(job=ran, dry=False))
    classes, basis, ran_basis = {
        MODE_ROLLED_BACK: (EXECUTE_CLASSES, BASIS_ROLLED_BACK, BASIS_RAN_ROLLED_BACK),
        MODE_CASE_DATASETS: (CASE_DATASET_CLASSES, BASIS_CASE_DATASETS, BASIS_RAN_CASE_DATASETS),
        MODE_TIMED: (TIMED_CLASSES, BASIS_TIMED, BASIS_RAN),
    }.get(pending.mode, (EXECUTE_CLASSES, BASIS_EXECUTE, BASIS_RAN))
    if ran.errors:
        judged: Observation = decided(
            errors=list(ran.errors),
            classes=classes,
            basis=basis,
            parse_only=False,
            said=_normaliser(sql=sql, names=names, line_offset=int(wrapped)),
        )
        return replace(judged, jobs=jobs, mode=pending.mode)
    return Observation(
        phase=ACCEPTED,
        basis=ran_basis,
        engine_code=None,
        parse_only_code=None,
        message=None,
        mode=pending.mode,
        jobs=jobs,
    )
