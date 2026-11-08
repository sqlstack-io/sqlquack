"""One referee run over case files, and the comparison of two runs."""

from __future__ import annotations

import json
from collections import Counter
from pathlib import Path
from typing import cast

from scripts.duckdb_referee._helpers.receipts import (
    canonical,
    digest,
    engine_sha256,
    now,
    raw_record,
    receipt,
    run_audit,
    write_jsonl,
)
from scripts.duckdb_referee.classes.referee import Referee
from scripts.duckdb_referee.constants import CAPTURE, COMPARED_KEYS, RUN_PREFIX
from scripts.duckdb_referee.exceptions import RefereeError
from scripts.duckdb_referee.models import Observation
from scripts.duckdb_referee.types import Row


def read_cases(paths: list[Path]) -> list[Row]:
    """Every case of the files, refusing one whose capture this referee does not run."""
    cases: list[Row] = []
    for path in paths:
        cases += [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    wrong: list[str] = [str(case["case_id"]) for case in cases if case["capture"] != CAPTURE]
    if wrong:
        raise RefereeError(f"cases with a capture other than {CAPTURE}: {wrong[:5]}")
    return cases


def observe_all(cases: list[Row]) -> list[tuple[Row, Observation, str]]:
    """Each case with what the engine did and when it was recorded."""
    observed: list[tuple[Row, Observation, str]] = []
    with Referee() as referee:
        for case in cases:
            observation: Observation = referee.observe(
                case=str(case["case_id"]),
                sql=str(case["sql"]),
                session=dict(cast("dict[str, str]", case["session"])),
                setup=list(cast("list[str]", case["setup"])),
            )
            observed.append((case, observation, now()))
    return observed


def receipts_of(*, observed: list[tuple[Row, Observation, str]], context: Row) -> list[Row]:
    """The published receipts, each bound to its raw record and the run audit."""
    digests: list[str] = list(cast("list[str]", context["raw_digests"]))
    found: list[Row] = []
    for (case, observation, at), raw_sha in zip(observed, digests, strict=True):
        found.append(
            receipt(
                case=case,
                observation=observation,
                context={
                    "run_id": context["run_id"],
                    "engine_sha256": context["engine_sha256"],
                    "recorded_at": at,
                    "raw_record_sha256": raw_sha,
                    "run_audit_sha256": context["run_audit_sha256"],
                    "packet_sha256": digest(canonical(case)),
                },
            )
        )
    return found


def record(*, cases: list[Row], out: Path, private: Path) -> Row:
    """Judge every case, then write the receipts, the raw records, the audit and the report."""
    engine: str = engine_sha256()
    run_id: str = RUN_PREFIX + now().replace(":", "").replace("-", "").replace("+0000", "Z")
    observed: list[tuple[Row, Observation, str]] = observe_all(cases)
    raws: list[Row] = [
        raw_record(case=case, observation=observation, recorded_at=at)
        for case, observation, at in observed
    ]
    raw_digests: list[str] = [digest(canonical(raw)) for raw in raws]
    phases: dict[str, int] = dict(sorted(Counter(o.phase for _, o, _ in observed).items()))
    audit: Row = run_audit(run_id=run_id, engine=engine, raw_digests=raw_digests, phases=phases)
    receipts: list[Row] = receipts_of(
        observed=observed,
        context={
            "run_id": run_id,
            "engine_sha256": engine,
            "raw_digests": raw_digests,
            "run_audit_sha256": digest(canonical(audit)),
        },
    )
    write_jsonl(path=private / f"{run_id}.raw.jsonl", rows=raws)
    (private / f"{run_id}.audit.json").write_text(canonical(audit) + "\n")
    write_jsonl(path=out, rows=receipts)
    report: Row = {"run_id": run_id, "receipts": len(receipts), "phases": phases, "problems": []}
    out.with_suffix(".report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    return report


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
