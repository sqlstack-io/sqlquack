"""One referee run over case files, and the comparison of two runs."""

from __future__ import annotations

import json
import random
import re
import threading
import time
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from scripts.databricks_referee._helpers.client import Client
from scripts.databricks_referee._helpers.judge import observe
from scripts.databricks_referee._helpers.receipts import (
    canonical,
    digest,
    now,
    raw_record,
    receipt,
    run_audit,
    version_of,
    write_jsonl,
)
from scripts.databricks_referee._helpers.scope import held_cases
from scripts.databricks_referee.constants import (
    CANCELLED_AT_DEADLINE,
    CAPTURE,
    CATALOG,
    COMPARED_KEYS,
    PHASE_ONLY_CASES,
    REFEREE_CLASS,
    REPLY_LOST,
    RUN_PREFIX,
    SCHEMA_PREFIX,
    SETUP_ERROR_CLASS,
    UNDECIDED_DEADLINE,
    UNDECIDED_REPLY_LOST,
    UNDECIDED_SETUP,
    VERSION_QUERY,
    VERSION_SECONDS,
    WORKERS,
)
from scripts.databricks_referee.exceptions import RefereeError
from scripts.databricks_referee.models import Failure, Observation, Run
from scripts.databricks_referee.types import Row

type Judged = tuple[Row, Observation | Failure, Row, str]


def read_cases(paths: list[Path]) -> list[Row]:
    """Every case of the files, refusing one whose capture this referee does not run."""
    cases: list[Row] = []
    for path in paths:
        cases += [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    wrong: list[str] = [str(case["case_id"]) for case in cases if case["capture"] != CAPTURE]
    if wrong:
        raise RefereeError(f"cases with a capture other than {CAPTURE}: {wrong[:5]}")
    if any(case["session"] for case in cases):
        raise RefereeError("a case sets session parameters, which this referee does not apply")
    return cases


def setup_text(setup: list[str]) -> list[str]:
    """The setup as one compound statement, so it costs one request."""
    if len(setup) < 2:  # noqa: PLR2004
        return setup
    return ["BEGIN " + " ; ".join(setup) + " ; END"]


class Versions:
    """current_version(), asked again once VERSION_SECONDS have passed; every answer is kept."""

    def __init__(self, client: Client) -> None:
        self.client: Client = client
        self.lock: threading.Lock = threading.Lock()
        self.current: Row = {}
        self.asked: float = 0.0
        self.seen: set[str] = set()

    def ask(self) -> Row:
        """The release now answering, as current_version() reports it."""
        version: Row = version_of(self.client.require(sql=VERSION_QUERY, schema=None))
        self.seen.add(canonical(version))
        return version

    def get(self) -> Row:
        with self.lock:
            if not self.current or time.monotonic() - self.asked > VERSION_SECONDS:
                self.current, self.asked = self.ask(), time.monotonic()
            return self.current


def judge_one(*, client: Client, versions: Versions, case: Row, schema: str) -> Judged:
    """One case's observation, the release that answered it and when it was recorded."""
    version: Row = versions.get()
    try:
        found: Observation | Failure = observe(
            client=client,
            schema=schema,
            sql=str(case["sql"]),
            setup=setup_text(list(case["setup"])),  # type: ignore[call-overload]
        )
    except RefereeError as error:
        found = Failure(error_class=REFEREE_CLASS, sqlstate="", message=str(error), parser=False)
    return case, found, version, now()


def leftover_schemas(*, client: Client, stem: str) -> list[str]:
    """This run's referee schemas still in the catalog; other runs' schemas are not its own."""
    reply: Row = client.require(sql=f"SHOW SCHEMAS IN {CATALOG} LIKE '{stem}_*'", schema=None)
    rows: list[list[str]] = dict(reply.get("result") or {}).get("data_array", [])  # type: ignore[call-overload]
    return [row[0] for row in rows]


def unmapped_report(failures: list[tuple[Row, Failure]]) -> Row:
    """Error classes with no reviewed phase: an example of each, and every case's first line."""
    by_class: dict[tuple[str, bool], list[tuple[Row, Failure]]] = {}
    for case, failure in failures:
        by_class.setdefault((failure.error_class, failure.parser), []).append((case, failure))
    return {
        "unmapped": [
            {
                "error_class": error_class,
                "raised_by_parser": parser,
                "sqlstate": found[0][1].sqlstate,
                "cases": len(found),
                "example_case": found[0][0]["case_id"],
                "example_sql": found[0][0]["sql"],
                "example_message": found[0][1].message[:600],
                "every_case": [
                    {
                        "case_id": case["case_id"],
                        "sql": case["sql"],
                        "message": failure.message.strip().splitlines()[0][:300],
                    }
                    for case, failure in found
                ],
            }
            for (error_class, parser), found in sorted(by_class.items())
        ]
    }


def new_run(*, out: Path, private: Path, seed: int | None) -> Run:
    """A new run's id and case-schema stem, both from the time it starts."""
    stamp: str = now().replace(":", "").replace("-", "").replace("+0000", "Z")
    return Run(
        run_id=RUN_PREFIX + stamp,
        schema_stem=SCHEMA_PREFIX + stamp.replace(".", "_").lower(),
        out=out,
        private=private,
        seed=seed,
    )


def case_order(count: int, seed: int | None) -> list[int]:
    """The cases' indexes in the order they run: as given, or shuffled by the seed."""
    order: list[int] = list(range(count))
    if seed is not None:
        random.Random(seed).shuffle(order)
    return order


def record(*, cases: list[Row], out: Path, private: Path, seed: int | None) -> Row:
    """Judge every case in its own schema; write receipts only when every phase is reviewed."""
    run: Run = new_run(out=out, private=private, seed=seed)
    held: list[Row] = held_cases(cases)
    if held:
        private.mkdir(parents=True, exist_ok=True)
        report_path: Path = private / f"{run.run_id}.held.json"
        report_path.write_text(json.dumps({"run_id": run.run_id, "held": held}, indent=2) + "\n")
        raise RefereeError(
            f"{len(held)} cases change account- or catalog-level state or cannot be placed in "
            f"the case schema; see {report_path}; nothing was run"
        )
    client: Client = Client()
    versions: Versions = Versions(client)
    with ThreadPoolExecutor(max_workers=WORKERS) as pool:
        judged: list[Judged] = list(
            pool.map(
                lambda index: judge_one(
                    client=client,
                    versions=versions,
                    case=cases[index],
                    schema=f"{run.schema_stem}_{index}",
                ),
                case_order(len(cases), seed),
            )
        )
    return finish(client=client, versions=versions, judged=judged, run=run)


def undecided_reason(case: Row, failure: Failure) -> str | None:
    """Why the case was left undecided with no phase observed, or None when it was not: a
    statement was cancelled at the deadline or its reply was lost, or one of the case's own setup
    statements failed (named by the error class its message carries), so the case's statement
    never ran. The reasons are listed in the run summary, so a setup that fails on every run is
    seen rather than left undecided unnoticed."""
    if failure.error_class != REFEREE_CLASS:
        return None
    message: str = failure.message
    if CANCELLED_AT_DEADLINE.match(message):
        return UNDECIDED_DEADLINE
    if message.startswith(REPLY_LOST):
        return UNDECIDED_REPLY_LOST
    for text in setup_text(list(case["setup"])):  # type: ignore[call-overload]
        prefix: str = f"{text[:200]!r} failed: "
        if message.startswith(prefix):
            named: re.Match[str] | None = SETUP_ERROR_CLASS.search(message, len(prefix))
            return UNDECIDED_SETUP + (f": {named.group(1)}" if named else "")
    return None


def finish(*, client: Client, versions: Versions, judged: list[Judged], run: Run) -> Row:
    """The end of every run: the release check, the unmapped-class report, the leftover-schema
    check, and the receipts, written only when all three pass. An undecided case (see
    `undecided_reason`) gets no receipt and is listed, so its form stays unverified."""
    run_id: str = run.run_id
    private: Path = run.private
    out: Path = run.out
    versions.ask()
    if len(versions.seen) != 1:
        raise RefereeError(
            f"the warehouse release changed during the run: {sorted(versions.seen)}; "
            "no receipt was written"
        )
    failures: list[tuple[Row, Failure]] = [
        (case, found)
        for case, found, _, _ in judged
        if isinstance(found, Failure) and undecided_reason(case, found) is None
    ]
    reasons: dict[str, str] = {
        str(case["case_id"]): reason
        for case, found, _, _ in judged
        if isinstance(found, Failure) and (reason := undecided_reason(case, found)) is not None
    }
    undecided: list[str] = sorted(reasons)
    private.mkdir(parents=True, exist_ok=True)
    if failures:
        report: Row = {"run_id": run_id, **unmapped_report(failures)}
        (private / f"{run_id}.unmapped.json").write_text(json.dumps(report, indent=2) + "\n")
    left: list[str] = leftover_schemas(client=client, stem=run.schema_stem)
    if left:
        raise RefereeError(f"referee schemas left behind: {left[:5]}")
    if failures:
        raise RefereeError(
            f"{len(failures)} cases failed with error classes no reviewed row maps to a phase; "
            f"see {private / f'{run_id}.unmapped.json'}; no receipt was written"
        )
    unidentified: list[str] = sorted(
        str(case["case_id"])
        for case, found, _, _ in judged
        if isinstance(found, Observation) and not found.statement_id
    )
    if unidentified:
        raise RefereeError(f"observations without a statement id: {unidentified[:5]}")
    observed: list[tuple[Row, Observation, Row, str]] = sorted(
        ((c, o, v, at) for c, o, v, at in judged if isinstance(o, Observation)),
        key=lambda item: str(item[0]["case_id"]),
    )
    raws: list[Row] = [
        raw_record(case=case, observation=found, version=version, recorded_at=at)
        for case, found, version, at in observed
    ]
    raw_digests: list[str] = [digest(canonical(raw)) for raw in raws]
    phases: Row = dict(sorted(Counter(o.phase for _, o, _, _ in observed).items()))
    versions: list[Row] = [json.loads(v) for v in sorted({canonical(v) for _, _, v, _ in observed})]
    audit: Row = run_audit(run_id=run_id, versions=versions, raw_digests=raw_digests, phases=phases)
    audit_sha: str = digest(canonical(audit))
    receipts: list[Row] = [
        receipt(
            case=case,
            observation=found,
            context={
                "run_id": run_id,
                "engine_sha256": digest(canonical(version)),
                "recorded_at": at,
                "raw_record_sha256": raw_sha,
                "run_audit_sha256": audit_sha,
                "packet_sha256": digest(canonical(case)),
            },
        )
        for (case, found, version, at), raw_sha in zip(observed, raw_digests, strict=True)
    ]
    write_jsonl(path=private / f"{run_id}.raw.jsonl", rows=raws)
    (private / f"{run_id}.audit.json").write_text(canonical(audit) + "\n")
    write_jsonl(path=out / f"{run_id}.jsonl", rows=receipts)
    summary: Row = {
        "run_id": run_id,
        "receipts": len(receipts),
        "undecided": undecided,
        "undecided_reasons": dict(sorted(reasons.items())),
        "phases": phases,
        "shuffle_seed": run.seed,
        "current_versions": versions,
        "problems": [],
    }
    (out / f"{run_id}.report.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    return summary


def by_case(path: Path) -> dict[str, Row]:
    """A receipt file's receipts by case id."""
    rows: list[Row] = [json.loads(line) for line in path.read_text().splitlines()]
    return {str(row["case_id"]): row for row in rows}


def compare(*, first: Path, second: Path) -> list[str]:
    """Cases whose phase, code, parse-only code or capture differ between two runs; a case in
    PHASE_ONLY_CASES is compared by its phase alone."""
    one: dict[str, Row] = by_case(first)
    two: dict[str, Row] = by_case(second)
    differ: list[str] = sorted(set(one) ^ set(two))
    for case_id in sorted(set(one) & set(two)):
        keys: tuple[str, ...] = ("phase",) if case_id in PHASE_ONLY_CASES else COMPARED_KEYS
        if any(one[case_id][key] != two[case_id][key] for key in keys):
            differ.append(case_id)
    print(f"{len(one)} and {len(two)} receipts; {len(differ)} differ")
    return differ
