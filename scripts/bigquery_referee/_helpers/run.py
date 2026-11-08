"""One referee run over case files, a one-off probe, and the comparison of two runs."""

from __future__ import annotations

import json
import re
import subprocess
from collections import Counter
from collections.abc import Generator
from concurrent.futures import Future, ThreadPoolExecutor
from contextlib import contextmanager
from pathlib import Path

from scripts.bigquery_referee._helpers.jobs import (
    bytes_of,
    create_dataset,
    drop_dataset,
    execute,
    run_state,
    said,
)
from scripts.bigquery_referee._helpers.judge import observe_dry, observe_execution
from scripts.bigquery_referee._helpers.names import (
    case_names,
    leaks,
    normalise,
    run_names,
    substitute,
)
from scripts.bigquery_referee._helpers.receipts import (
    canonical,
    digest,
    now,
    raw_record,
    receipt,
    run_audit,
    write_jsonl,
)
from scripts.bigquery_referee._helpers.values import field, items_of
from scripts.bigquery_referee.constants import (
    BYTES_KEY,
    CAPTURE,
    CASE_BYTES_KEY,
    COMPARED_KEYS,
    END_STATE_KEY,
    ENGINE_IDENTITY,
    FIXTURE_STATEMENT,
    GIT_HEAD,
    GIT_STATUS,
    INTERNAL_HELD,
    INTERNAL_LIMIT,
    JOBS_KEY,
    MODE_CASE_DATASETS,
    MODE_HELD,
    MODE_TIMED,
    REFEREE_COMMIT_KEY,
    REFEREE_DIRTY,
    REFEREE_PATHS,
    RUN_PREFIX,
    SPEND_STOP_BYTES,
    START_STATE_KEY,
    TOKEN_DATASET,
    TOKEN_PATTERN,
    WORKERS,
)
from scripts.bigquery_referee.exceptions import RefereeError, StopRun
from scripts.bigquery_referee.models import Fixture, JobResult, Observation, RunNames, RunPaths
from scripts.bigquery_referee.types import Observed, Row, Transport
from scripts.syntax_obligations_bigquery._helpers.fixtures import case_setup


def read_cases(paths: list[Path]) -> list[Row]:
    """Every case of the files, refusing one whose capture or setup this referee does not run."""
    cases: list[Row] = []
    for path in paths:
        cases += [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    wrong: list[str] = [str(case["case_id"]) for case in cases if case["capture"] != CAPTURE]
    if wrong:
        raise RefereeError(f"cases with a capture other than {CAPTURE}: {wrong[:5]}")
    sessions: list[str] = [str(case["case_id"]) for case in cases if case["session"]]
    if sessions:
        raise RefereeError(f"cases with a session, which this referee does not run: {sessions[:5]}")
    unbound: list[str] = [
        str(case["case_id"])
        for case in cases
        if digest(canonical(case["setup"])) != case["setup_sha256"]
    ]
    if unbound:
        raise RefereeError(f"cases whose setup_sha256 does not bind their setup: {unbound[:5]}")
    return cases


def fixtures_for(cases: list[Row]) -> list[Fixture]:
    """The distinct setup statements of the cases, in first-use order, each a declared fixture."""
    statements: list[str] = []
    for case in cases:
        statements += [str(s) for s in items_of(case["setup"]) if str(s) not in statements]
    fixtures: list[Fixture] = []
    for statement in statements:
        shaped: re.Match[str] | None = re.match(FIXTURE_STATEMENT, statement)
        if shaped is None:
            raise RefereeError(f"a setup statement that is not a declared fixture: {statement}")
        fixtures.append(Fixture(name=shaped.group(1), ddl=statement))
    return fixtures


def build_fixtures(*, transport: Transport, fixtures: list[Fixture], names: RunNames) -> None:
    """Create each fixture with DDL in the run dataset; one naming anything else is refused."""
    for fixture in fixtures:
        tokens: set[str] = set(re.findall(TOKEN_PATTERN, fixture.ddl))
        if tokens != {TOKEN_DATASET}:
            raise RefereeError(f"fixture {fixture.name} names more than the run dataset: {tokens}")
        made: JobResult = execute(
            transport=transport, sql=substitute(sql=fixture.ddl, names=names), names=names
        )
        if made.errors:
            stated: str = said(errors=list(made.errors), names=names)
            raise RefereeError(f"fixture {fixture.name} failed: {stated}")


@contextmanager
def run_datasets(*, transport: Transport, names: RunNames) -> Generator[None]:
    """The run's two datasets, created for the run and dropped with their contents after it."""
    created: list[str] = []
    try:
        for dataset in (names.dataset, names.dataset2):
            create_dataset(transport=transport, dataset=dataset, names=names)
            created.append(dataset)
        yield
    finally:
        failed: list[str] = []
        for dataset in reversed(created):
            try:
                drop_dataset(transport=transport, dataset=dataset, names=names)
            except RefereeError as error:
                failed.append(normalise(text=str(error), names=names))
        if failed:
            raise RefereeError("run datasets not dropped: " + "; ".join(failed))


def dry_all(
    *, cases: list[Row], transport: Transport, names: RunNames, workers: int
) -> list[Observation]:
    """Every case's dry run on a worker pool, in case order; a stop error stops the pool."""
    pool: ThreadPoolExecutor = ThreadPoolExecutor(max_workers=workers)
    try:
        futures: list[Future[Observation]] = [
            pool.submit(observe_dry, case=case, transport=transport, names=names) for case in cases
        ]
        return [future.result() for future in futures]
    finally:
        pool.shutdown(wait=True, cancel_futures=True)


def observe_all(
    *,
    cases: list[Row],
    transport: Transport,
    names: RunNames,
    start: Row,
    workers: int = WORKERS,
) -> list[Observed]:
    """Dry runs in parallel, then the executions one at a time, the state read after each."""
    dried: list[Observation] = dry_all(
        cases=cases, transport=transport, names=names, workers=workers
    )
    internal: list[str] = [
        str(case["case_id"])
        for case, o in zip(cases, dried, strict=True)
        if o.held == INTERNAL_HELD
    ]
    if len(internal) > INTERNAL_LIMIT:
        raise StopRun(f"{len(internal)} dry runs gave internalError on every retry: {internal[:5]}")
    observed: list[Observed] = []
    own_index: int = 0
    billed: int = 0
    for case, pending in zip(cases, dried, strict=True):
        if not pending.mode or pending.mode == MODE_HELD:
            observed.append((case, pending, now()))
            continue
        done: Observation
        if pending.mode == MODE_CASE_DATASETS:
            own_index += 1
            done = observe_in_case_datasets(
                case=case, pending=pending, transport=transport, names=names, index=own_index
            )
        else:
            done = observe_execution(case=case, pending=pending, transport=transport, names=names)
        observed.append((case, done, now()))
        billed += executed_billed(done)
        if billed > SPEND_STOP_BYTES:
            raise StopRun(f"executions billed {billed} bytes, past the run's {SPEND_STOP_BYTES}")
        after: Row = run_state(transport=transport, names=names)
        if after != start:
            raise RefereeError(f"{case['case_id']} changed the run datasets: {start} -> {after}")
    return observed


def observe_in_case_datasets(
    *, case: Row, pending: Observation, transport: Transport, names: RunNames, index: int
) -> Observation:
    """Execute a case in two datasets of its own, with only its fixtures, dropped after it."""
    own: RunNames = case_names(names=names, index=index)
    with run_datasets(transport=transport, names=own):
        build_fixtures(transport=transport, fixtures=fixtures_for([case]), names=own)
        return observe_execution(case=case, pending=pending, transport=transport, names=own)


def executed_billed(observation: Observation) -> int:
    """Bytes billed by the case's executed jobs; dry runs bill nothing."""
    return bytes_of(list(observation.jobs))["executed_bytes_billed"]


def unmapped_report(observed: list[Observed]) -> list[str]:
    """Each unmapped error class with its count and one case, sorted."""
    counts: Counter[str] = Counter()
    sample: dict[str, str] = {}
    for case, observation, _ in observed:
        for key in observation.unmapped:
            counts[key] += 1
            sample.setdefault(key, str(case["case_id"]))
    return [f"{key} x{count}: {sample[key]}" for key, count in sorted(counts.items())]


def receipts_of(*, observed: list[Observed], digests: dict[str, str], context: Row) -> list[Row]:
    """The published receipts of the scored cases, each bound to its raw record and the audit."""
    return [
        receipt(
            case=case,
            observation=observation,
            context={
                "run_id": context["run_id"],
                "recorded_at": at,
                "raw_record_sha256": digests[str(case["case_id"])],
                "run_audit_sha256": context["run_audit_sha256"],
                "packet_sha256": digest(canonical(case)),
            },
        )
        for case, observation, at in observed
        if not observation.held
    ]


def check_no_leaks(*, rows: list[Row], names: RunNames) -> None:
    """Refuse to publish text that names the project, a run dataset, a job, a URL or an email."""
    leaked: list[str] = [
        str(row.get("case_id", "report")) for row in rows if leaks(text=canonical(row), names=names)
    ]
    if leaked:
        raise RefereeError(f"published text would name the run's resources: {leaked[:10]}")


def referee_commit(paths: list[Path]) -> str:
    """The commit the run is made against, marked when the referee or the cases differ from it."""
    head: str = subprocess.run(GIT_HEAD, capture_output=True, text=True, check=True).stdout.strip()
    changed: str = subprocess.run(
        [*GIT_STATUS, "--", *REFEREE_PATHS, *(str(path) for path in paths)],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    return head + REFEREE_DIRTY if changed else head


def run_id_now() -> str:
    """A run id from the current time, as the SQL Server referee writes it."""
    return RUN_PREFIX + now().replace(":", "").replace("-", "").replace("+0000", "Z")


def record(
    *,
    cases: list[Row],
    paths: RunPaths,
    transport: Transport,
    project: str,
    workers: int = WORKERS,
) -> Row:
    """Judge every case in its own run datasets, then write receipts, raw records, audit, report."""
    referee: str = referee_commit(list(paths.cases))
    run_id: str = run_id_now()
    names: RunNames = run_names(run_id=run_id, project=project)
    fixtures: list[Fixture] = fixtures_for(cases)
    with run_datasets(transport=transport, names=names):
        build_fixtures(transport=transport, fixtures=fixtures, names=names)
        start: Row = run_state(transport=transport, names=names)
        observed: list[Observed] = observe_all(
            cases=cases, transport=transport, names=names, start=start, workers=workers
        )
        end: Row = run_state(transport=transport, names=names)
    raws: list[Row] = [
        raw_record(
            case=case,
            submitted=substitute(sql=str(case["sql"]), names=names),
            observation=observation,
            recorded_at=at,
        )
        for case, observation, at in observed
    ]
    private: Path = paths.private
    write_jsonl(path=private / f"{run_id}.raw.jsonl", rows=raws)
    unmapped: list[str] = unmapped_report(observed)
    if unmapped:
        raise RefereeError("errors with no reviewed class:\n" + "\n".join(unmapped))
    raw_digests: list[tuple[str, str]] = [
        (str(field(value=raw, path=("case", "case_id"))), digest(canonical(raw))) for raw in raws
    ]
    scored: list[Observation] = [o for _, o, _ in observed if not o.held]
    phases: dict[str, int] = dict(sorted(Counter(o.phase for o in scored).items()))
    held: dict[str, int] = dict(sorted(Counter(o.held for _, o, _ in observed if o.held).items()))
    jobs: list[Row] = []
    for _, observation, _ in observed:
        jobs += observation.jobs
    spent: dict[str, int] = bytes_of(jobs)
    case_bytes: Row = {
        str(case["case_id"]): executed_billed(observation)
        for case, observation, _ in sorted(observed, key=lambda item: str(item[0]["case_id"]))
        if observation.mode in (MODE_CASE_DATASETS, MODE_TIMED)
    }
    counts: Row = {"phases": phases, "held": held, BYTES_KEY: spent}
    audit: Row = run_audit(
        run_id=run_id,
        names=names,
        state=start,
        raw_digests=[d for _, d in raw_digests],
        counts=counts,
    )
    receipts: list[Row] = receipts_of(
        observed=observed,
        digests=dict(raw_digests),
        context={
            "run_id": run_id,
            "run_audit_sha256": digest(canonical(audit)),
        },
    )
    report: Row = {
        "run_id": run_id,
        "receipts": len(receipts),
        "phases": phases,
        "held": held,
        "problems": [],
        "engine": ENGINE_IDENTITY,
        "fixtures": [fixture.name for fixture in fixtures],
        BYTES_KEY: spent,
        CASE_BYTES_KEY: case_bytes,
        REFEREE_COMMIT_KEY: referee,
        START_STATE_KEY: start,
        END_STATE_KEY: end,
    }
    check_no_leaks(rows=[*receipts, report], names=names)
    (private / f"{run_id}.audit.json").write_text(canonical(audit) + "\n")
    write_jsonl(
        path=private / f"{run_id}.held.jsonl",
        rows=[{"case_id": c["case_id"], "held": o.held} for c, o, _ in observed if o.held],
    )
    write_jsonl(path=paths.out, rows=receipts)
    paths.out.with_suffix(".report.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n"
    )
    return report


def probe(*, sql: str, family: str, transport: Transport, project: str) -> Row:
    """One text judged as a case of `family`, in its own run datasets with the fixtures it names."""
    case: Row = {"case_id": f"bq.{family}.probe", "sql": sql, "setup": case_setup(sql)}
    names: RunNames = run_names(run_id=run_id_now(), project=project)
    with run_datasets(transport=transport, names=names):
        build_fixtures(transport=transport, fixtures=fixtures_for([case]), names=names)
        start: Row = run_state(transport=transport, names=names)
        ((_, observation, _),) = observe_all(
            cases=[case], transport=transport, names=names, start=start, workers=1
        )
    shown: Row = dict(observation.__dict__)
    shown.pop(JOBS_KEY)
    shown[BYTES_KEY] = bytes_of(list(observation.jobs))
    return json.loads(normalise(text=canonical(shown), names=names))


def by_case(path: Path) -> dict[str, Row]:
    """A receipt file's receipts by case id."""
    rows: list[Row] = [json.loads(line) for line in path.read_text().splitlines()]
    return {str(row["case_id"]): row for row in rows}


def compare(*, first: Path, second: Path) -> list[str]:
    """Cases whose phase, code, parse-only code or capture differ between two runs."""
    one: dict[str, Row] = by_case(first)
    two: dict[str, Row] = by_case(second)
    differ: list[str] = sorted(set(one) ^ set(two))
    for case_id in sorted(set(one) & set(two)):
        if any(one[case_id][key] != two[case_id][key] for key in COMPARED_KEYS):
            differ.append(case_id)
    print(f"{len(one)} and {len(two)} receipts; {len(differ)} differ")
    return differ
