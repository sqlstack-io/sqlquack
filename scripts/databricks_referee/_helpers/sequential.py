"""The sequential phase: cases that change account- or catalog-level state, one at a time.

The account state of SNAPSHOT_KINDS is read before the first case. After each case, which is
judged in its own schema as in a parallel run, the state is read again: an object the case added
under a reviewed fixture name with a reviewed drop is dropped, and the state must then equal the
baseline. A removed or changed object, any other added object, a failed drop or a state that
differs from the baseline after cleanup stops the run at once, leaves the state as it is for
review, and writes no receipt.

Each step is journaled before it runs: the baseline first, then each case with its schema and the
fixture-named objects it may create, then the case once its state is settled. When a run is
stopped or freezes part way, `recover` reads the journal and drops only what the unsettled case
may have created: its schema, and objects it added that its statement names under a fixture name
with a reviewed drop. Anything else that differs from the baseline is reported and left alone.

Each settled line also holds the case's answer and the restored baseline, so `resume` continues a
stopped run: with the same referee source, the same cases in the same order and seed, and only
once the account state, after recovering the unsettled case, equals the journal's baseline.
"""

from __future__ import annotations

import json
import os
import urllib.parse
from dataclasses import asdict, replace
from datetime import datetime
from pathlib import Path
from typing import NoReturn

from scripts.databricks_referee._helpers import judge
from scripts.databricks_referee._helpers.client import Client
from scripts.databricks_referee._helpers.receipts import canonical, digest
from scripts.databricks_referee._helpers.run import (
    Judged,
    Versions,
    case_order,
    finish,
    judge_one,
    leftover_schemas,
    new_run,
)
from scripts.databricks_referee.constants import (
    CASE_SECONDS,
    DROP_BY_SCIM,
    DROPS,
    FIXTURE_NAMES,
    GROUPS_PATH,
    HISTORY_FAILED,
    HISTORY_PAGE,
    HISTORY_PATH,
    JOURNAL_BASELINE,
    JOURNAL_FAILURE,
    JOURNAL_FINISHED,
    JOURNAL_OBSERVATION,
    JOURNAL_PENDING,
    JOURNAL_RESTORED,
    JOURNAL_SETTLED,
    METASTORE_PATH,
    METASTORE_POLICY_ABSENT,
    METASTORE_POLICY_CASE,
    METASTORE_POLICY_DROP,
    METASTORE_POLICY_NAMES,
    PAGING_SCIM,
    PAGING_TOKEN,
    SCIM_PAGE,
    SNAPSHOT_KINDS,
    STATE_SUCCEEDED,
    STATEMENT_NAMES,
    STATEMENTS_PATH,
)
from scripts.databricks_referee.exceptions import RefereeError
from scripts.databricks_referee.models import Failure, Observation, Run, SnapshotKind
from scripts.databricks_referee.types import Row

# Each kind's objects by name, each object as its fingerprint fields.
type Snapshot = dict[str, dict[str, Row]]


def with_query(path: str, query: str) -> str:
    """The path with one more query parameter."""
    return f"{path}{'&' if '?' in path else '?'}{query}"


def listed(*, client: Client, kind: SnapshotKind, path: str) -> list[Row]:
    """Every object of one list, page by page."""
    items: list[Row] = []
    if kind.paging == PAGING_SCIM:
        start: int = 1
        while True:
            reply: Row = client._call(
                method="GET", path=with_query(path, f"startIndex={start}&count={SCIM_PAGE}")
            )
            page: list[Row] = list(reply.get(kind.items) or [])  # type: ignore[call-overload]
            items += page
            start += len(page)
            if not page or start > int(reply.get("totalResults") or 0):  # type: ignore[call-overload]
                return items
    token: str = ""
    while True:
        query: str = f"{PAGING_TOKEN}={urllib.parse.quote(token, safe='')}"
        reply = client._call(method="GET", path=with_query(path, query) if token else path)
        items += list(reply.get(kind.items) or [])  # type: ignore[call-overload]
        token = str(reply.get("next_page_token") or "")
        if not token:
            return items


def fingerprint(*, item: Row, fields: tuple[str, ...]) -> Row:
    """The fields of an object a change alters; a list is sorted, and `a.b` is the sorted `b` of
    each item of list `a`."""
    found: Row = {}
    for field in fields:
        head, _, tail = field.partition(".")
        value: object = item.get(head)
        if tail:
            value = sorted(str(dict(entry).get(tail)) for entry in list(value or []))  # type: ignore[call-overload]
        elif isinstance(value, list):
            value = sorted(value, key=canonical)
        found[field] = value
    return found


def snapshot(client: Client) -> Snapshot:
    """The account state of every snapshot kind, without the referee's own case schemas."""
    metastore: str = str(client._call(method="GET", path=METASTORE_PATH)["metastore_id"])
    state: Snapshot = {}
    for name, kind in SNAPSHOT_KINDS.items():
        path: str = kind.path.format(metastore_id=urllib.parse.quote(metastore, safe=""))
        objects: dict[str, Row] = {}
        for item in listed(client=client, kind=kind, path=path):
            key: str = str(item[kind.name])
            if kind.skip is not None and item.get(kind.skip[0]) == kind.skip[1]:
                continue
            if kind.skip_prefix is not None and key.startswith(kind.skip_prefix):
                continue
            if key in objects:
                raise RefereeError(f"two {name} objects are named {key!r}")
            objects[key] = fingerprint(item=item, fields=kind.fields)
        state[name] = objects
    return state


def difference(before: Snapshot, after: Snapshot) -> Row:
    """The objects added, removed and changed between two snapshots."""
    added: list[Row] = []
    removed: list[Row] = []
    changed: list[Row] = []
    for kind in SNAPSHOT_KINDS:
        one: dict[str, Row] = before.get(kind, {})
        two: dict[str, Row] = after.get(kind, {})
        added += [{"kind": kind, "name": n, "fields": two[n]} for n in sorted(set(two) - set(one))]
        removed += [
            {"kind": kind, "name": n, "fields": one[n]} for n in sorted(set(one) - set(two))
        ]
        changed += [
            {"kind": kind, "name": n, "before": one[n], "after": two[n]}
            for n in sorted(set(one) & set(two))
            if one[n] != two[n]
        ]
    return {"added": added, "removed": removed, "changed": changed}


def drop_of(added: Row) -> int | None:
    """The DROPS row of an added object, or None when its name or kind has no reviewed drop."""
    name: str = str(added["name"])
    if name.lower() not in FIXTURE_NAMES:
        return None
    variant: str | None = SNAPSHOT_KINDS[str(added["kind"])].variant
    value: object = dict(added["fields"]).get(variant) if variant else None  # type: ignore[call-overload]
    for index, (kind, kind_variant, _) in enumerate(DROPS):
        if kind == added["kind"] and kind_variant == value:
            return index
    return None


def quoted(name: str) -> str:
    """A name as a backquoted identifier."""
    return "`" + name.replace("`", "``") + "`"


def drop(*, client: Client, added: Row, index: int) -> Row:
    """Drop one added object by its reviewed drop; a failure stops the run."""
    statement: str = DROPS[index][2]
    if statement == DROP_BY_SCIM:
        group: str = urllib.parse.quote(str(dict(added["fields"])["id"]), safe="")  # type: ignore[call-overload]
        path: str = f"{GROUPS_PATH}/{group}"
        client._call(method="DELETE", path=path)
        return {"kind": added["kind"], "name": added["name"], "statement": f"DELETE {path}"}
    sql: str = statement.format(name=quoted(str(added["name"])))
    client.require(sql=sql, schema=None)
    return {"kind": added["kind"], "name": added["name"], "statement": sql}


def stop(*, run: Run, report: Row) -> NoReturn:
    """Write the private stop report and stop the run before any receipt is written."""
    run.private.mkdir(parents=True, exist_ok=True)
    path: Path = run.private / f"{run.run_id}.sequential-stop.json"
    path.write_text(json.dumps({"run_id": run.run_id, **report}, indent=2) + "\n")
    raise RefereeError(f"{report['reason']}; see {path}; no receipt was written")


def sweep_metastore_policies(*, client: Client, run: Run, case: Row | None) -> list[Row]:
    """Drop each fixture-named metastore policy, which no snapshot can see; stop on any answer but
    one saying the policy is absent or could not have been created."""
    answers: list[Row] = []
    for name in METASTORE_POLICY_NAMES:
        sql: str = METASTORE_POLICY_DROP.format(name=quoted(name))
        status: Row = dict(client.execute(sql=sql, schema=None)["status"])  # type: ignore[call-overload]
        message: str = str(dict(status.get("error") or {}).get("message", ""))  # type: ignore[call-overload]
        answer: Row = {"statement": sql, "state": status["state"], "message": message[:300]}
        answers.append(answer)
        if status["state"] == STATE_SUCCEEDED or not METASTORE_POLICY_ABSENT.match(message):
            reason: str = "a metastore policy drop did not answer that the policy is absent"
            stop(
                run=run,
                report={
                    "case_id": case["case_id"] if case else None,
                    "sql": case["sql"] if case else None,
                    "reason": reason,
                    "policy_drops": answers,
                },
            )
    return answers


def settle(*, client: Client, baseline: Snapshot, case: Row, run: Run) -> list[Row]:
    """Restore the baseline after one case and return what was dropped, or stop the run."""
    stopped: Row = {"case_id": case["case_id"], "sql": case["sql"]}
    found: Row = difference(baseline, snapshot(client))
    added: list[Row] = list(found["added"])  # type: ignore[call-overload]
    if found["removed"] or found["changed"]:
        reason: str = "the case removed or changed an object that existed before it"
        stop(run=run, report={**stopped, "reason": reason, "diff": found, "dropped": []})
    drops: list[tuple[int | None, Row]] = [(drop_of(entry), entry) for entry in added]
    planned: list[tuple[int, Row]] = [(index, entry) for index, entry in drops if index is not None]
    if len(planned) != len(drops):
        reason = "the case added an object with no reviewed fixture name or drop"
        stop(run=run, report={**stopped, "reason": reason, "diff": found, "dropped": []})
    if not planned:
        return []
    dropped: list[Row] = []
    for index, entry in sorted(planned, key=lambda pair: (pair[0], str(pair[1]["name"]))):
        try:
            dropped.append(drop(client=client, added=entry, index=index))
        except RefereeError as error:
            reason = f"a cleanup statement failed: {error}"
            stop(run=run, report={**stopped, "reason": reason, "diff": found, "dropped": dropped})
    left: Row = difference(baseline, snapshot(client))
    if any(left.values()):
        reason = "the account state differs from the baseline after cleanup"
        stop(run=run, report={**stopped, "reason": reason, "diff": left, "dropped": dropped})
    return dropped


def may_create(sql: str) -> list[str]:
    """The fixture names a statement writes, lower-cased: the objects it may create."""
    names: set[str] = set()
    for quoted_name, bare in STATEMENT_NAMES.findall(sql):
        name: str = (quoted_name.replace("``", "`") or bare).lower()
        if name in FIXTURE_NAMES:
            names.add(name)
    return sorted(names)


def journal_path(*, private: Path, run_id: str) -> Path:
    return private / f"{run_id}.sequential-journal.jsonl"


def journal(path: Path, entry: Row) -> None:
    """Append one journal line and flush it to disk before the step it names runs."""
    with path.open("a", encoding="utf-8") as file:
        file.write(json.dumps(entry, sort_keys=True) + "\n")
        file.flush()
        os.fsync(file.fileno())


def recover(*, path: Path, private: Path) -> Row:
    """Restore what the journal's unsettled case may have created, and report what is left."""
    entries: list[Row] = [json.loads(line) for line in path.read_text().splitlines() if line]
    first: Row = entries[0]
    if first["event"] != JOURNAL_BASELINE:
        raise RefereeError(f"{path} does not start with a baseline")
    run_id: str = str(first["run_id"])
    settled: set[str] = {str(e["case_id"]) for e in entries if e["event"] == JOURNAL_SETTLED}
    pending: list[Row] = [
        e for e in entries if e["event"] == JOURNAL_PENDING and str(e["case_id"]) not in settled
    ]
    report: Row = {"run_id": run_id, "unsettled": [e["case_id"] for e in pending], "dropped": []}
    if entries[-1]["event"] == JOURNAL_FINISHED:
        report["finished"] = True
    client: Client = Client()
    dropped: list[Row] = report["dropped"]  # type: ignore[assignment]
    for schema in leftover_schemas(client=client, stem=str(first["schema_stem"])):
        judge.drop(client=client, schema=schema)
        dropped.append({"kind": "case schema", "name": schema})
    recorded: set[str] = {name for e in pending for name in e["may_create"]}  # type: ignore[union-attr]
    baseline: Snapshot = first["baseline"]  # type: ignore[assignment]
    found: Row = difference(baseline, snapshot(client))
    for entry in list(found["added"]):  # type: ignore[call-overload]
        index: int | None = drop_of(entry)
        if index is not None and str(entry["name"]).lower() in recorded:
            dropped.append(drop(client=client, added=entry, index=index))
    run: Run = Run(
        run_id=run_id,
        schema_stem=str(first["schema_stem"]),
        out=private,
        private=private,
        seed=None,
    )
    if any(METASTORE_POLICY_CASE.search(str(e["sql"])) for e in pending):
        report["policy_drops"] = sweep_metastore_policies(client=client, run=run, case=None)
    report["left"] = difference(baseline, snapshot(client))
    private.mkdir(parents=True, exist_ok=True)
    (private / f"{run_id}.sequential-recover.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n"
    )
    return report


def referee_source() -> str:
    """The digest of the referee's own source, so a resumed run uses the code it started with."""
    root: Path = Path(__file__).resolve().parents[1]
    files: list[Path] = sorted(p for p in root.rglob("*.py") if "__pycache__" not in p.parts)
    return digest(canonical({str(p.relative_to(root)): p.read_text() for p in files}))


def selection(cases: list[Row]) -> str:
    """The digest of the cases as given, in order, each whole."""
    return digest(canonical([digest(canonical(case)) for case in cases]))


def answer(judged: Judged) -> Row:
    """A judged case's answer as a journal line holds it."""
    _, found, version, at = judged
    kind: str = JOURNAL_OBSERVATION if isinstance(found, Observation) else JOURNAL_FAILURE
    return {"kind": kind, "found": asdict(found), "version": version, "recorded_at": at}


def judged_from(case: Row, entry: Row) -> Judged:
    """A settled journal line's case answer, as it was judged."""
    found: Row = dict(entry["answer"])  # type: ignore[call-overload]
    model: type[Observation] | type[Failure] = (
        Observation if found["kind"] == JOURNAL_OBSERVATION else Failure
    )
    return case, model(**dict(found["found"])), dict(found["version"]), str(found["recorded_at"])  # type: ignore[call-overload]


def judge_remaining(
    *, cases: list[Row], run: Run, baseline: Snapshot, log: Path, judged: list[Judged]
) -> Row:
    """Judge the cases not yet settled, one at a time, each settled line journaled with its
    answer once the baseline is restored; then the final sweep, the final diff and the receipts."""
    client: Client = Client()
    versions: Versions = Versions(client)
    versions.seen |= {canonical(version) for _, _, version, _ in judged}
    cleanup: dict[str, list[Row]] = {}
    for index in case_order(len(cases), run.seed)[len(judged) :]:
        case: Row = cases[index]
        schema: str = f"{run.schema_stem}_{index}"
        try:
            client.check()
        except RefereeError as error:
            raise RefereeError(
                f"stopped before {case['case_id']}, which has not started: {error}; "
                "resume continues from it"
            ) from error
        journal(
            log,
            {
                "event": JOURNAL_PENDING,
                "case_id": case["case_id"],
                "sql": case["sql"],
                "schema": schema,
                "may_create": may_create(str(case["sql"])),
            },
        )
        one: Judged = judge_one(client=client, versions=versions, case=case, schema=schema)
        judged.append(one)
        dropped: list[Row] = settle(client=client, baseline=baseline, case=case, run=run)
        if METASTORE_POLICY_CASE.search(str(case["sql"])):
            dropped += sweep_metastore_policies(client=client, run=run, case=case)
        cleanup[str(case["case_id"])] = dropped
        journal(
            log,
            {
                "event": JOURNAL_SETTLED,
                "case_id": case["case_id"],
                "dropped": dropped,
                "answer": answer(one),
                "account": JOURNAL_RESTORED,
            },
        )
    cleanup["final metastore policy sweep"] = sweep_metastore_policies(
        client=client, run=run, case=None
    )
    journal(log, {"event": JOURNAL_FINISHED})
    final: Row = difference(baseline, snapshot(client))
    run.private.mkdir(parents=True, exist_ok=True)
    (run.private / f"{run.run_id}.sequential-cleanup.json").write_text(
        json.dumps(cleanup, indent=2, sort_keys=True) + "\n"
    )
    (run.private / f"{run.run_id}.sequential-diff.json").write_text(
        json.dumps(final, indent=2) + "\n"
    )
    if any(final.values()):
        reason: str = "the account state differs from the baseline after the last case"
        stop(run=run, report={"case_id": None, "sql": None, "reason": reason, "diff": final})
    return finish(client=client, versions=versions, judged=judged, run=run)


def record_sequential(*, cases: list[Row], out: Path, private: Path, seed: int | None) -> Row:
    """Judge the cases one at a time, held cases included, restoring the account state after each;
    write receipts as a parallel run does, only when every case left the baseline restored."""
    run: Run = new_run(out=out, private=private, seed=seed)
    baseline: Snapshot = snapshot(Client())
    named: list[Row] = [
        {"kind": kind, "name": name}
        for kind, objects in baseline.items()
        for name in objects
        if name.lower() in FIXTURE_NAMES
    ]
    if named:
        reason: str = "the account already holds objects with fixture names, before any case ran"
        stop(run=run, report={"case_id": None, "sql": None, "reason": reason, "objects": named})
    private.mkdir(parents=True, exist_ok=True)
    log: Path = journal_path(private=private, run_id=run.run_id)
    journal(
        log,
        {
            "event": JOURNAL_BASELINE,
            "run_id": run.run_id,
            "schema_stem": run.schema_stem,
            "baseline": baseline,
            "seed": seed,
            "selection": selection(cases),
            "referee": referee_source(),
        },
    )
    return judge_remaining(cases=cases, run=run, baseline=baseline, log=log, judged=[])


def resume_sequential(
    *, path: Path, cases: list[Row], out: Path, private: Path, seed: int | None
) -> Row:
    """Continue a stopped sequential run from its journal, or refuse: when the referee source,
    the cases, their order or the seed differ, when the run finished, or when the account state
    differs from the baseline once the unsettled case is recovered."""
    entries: list[Row] = [json.loads(line) for line in path.read_text().splitlines() if line]
    first: Row = entries[0]
    if first["event"] != JOURNAL_BASELINE:
        raise RefereeError(f"{path} does not start with a baseline")
    if entries[-1]["event"] == JOURNAL_FINISHED:
        raise RefereeError(f"{path} is a finished run; there is nothing to resume")
    checks: list[tuple[str, object, object]] = [
        ("the referee source", first.get("referee"), referee_source()),
        ("the cases or their order", first.get("selection"), selection(cases)),
        ("the shuffle seed", first.get("seed"), seed),
    ]
    for what, then, now in checks:
        if then != now:
            raise RefereeError(f"{what} differ from the stopped run's; it is not resumed")
    settled: list[Row] = [e for e in entries if e["event"] == JOURNAL_SETTLED]
    order: list[int] = case_order(len(cases), seed)
    judged: list[Judged] = []
    for index, entry in zip(order, settled, strict=False):
        if entry["case_id"] != cases[index]["case_id"] or "answer" not in entry:
            raise RefereeError(f"{path} settled {entry['case_id']} out of order or without answer")
        judged.append(judged_from(cases[index], entry))
    if len(judged) != len(settled):
        raise RefereeError(f"{path} settled more cases than were selected")
    recovered: Row = recover(path=path, private=private)
    baseline: Snapshot = first["baseline"]  # type: ignore[assignment]
    found: Row = difference(baseline, snapshot(Client()))
    if any(dict(recovered["left"]).values()) or any(found.values()):  # type: ignore[call-overload]
        raise RefereeError(
            "the account state differs from the stopped run's baseline after recovery; "
            f"see {private / (str(first['run_id']) + '.sequential-recover.json')}; not resumed"
        )
    run: Run = Run(
        run_id=str(first["run_id"]),
        schema_stem=str(first["schema_stem"]),
        out=out,
        private=private,
        seed=seed,
    )
    return judge_remaining(cases=cases, run=run, baseline=baseline, log=path, judged=judged)


def refinish_sequential(
    *, path: Path, cases: list[Row], out: Path, private: Path, seed: int | None
) -> Row:
    """The receipts of a finished sequential run whose account state ended as its baseline, its
    failures judged again with the current reviewed rows: a failure a row now decides becomes an
    observation of its recorded statement, found in the warehouse's history. Refuses when the
    cases, their order or the seed differ, or any case is unsettled. The run's answers are
    reused as recorded; only their phase decision is the current source's."""
    entries: list[Row] = [json.loads(line) for line in path.read_text().splitlines() if line]
    first: Row = entries[0]
    if first["event"] != JOURNAL_BASELINE or entries[-1]["event"] != JOURNAL_FINISHED:
        raise RefereeError(f"{path} is not a finished sequential run")
    run_id: str = str(first["run_id"])
    final: Row = json.loads((private / f"{run_id}.sequential-diff.json").read_text())
    if any(final.values()):
        raise RefereeError(f"{run_id} ended with the account state unlike its baseline")
    for what, then, now in (
        ("the cases or their order", first.get("selection"), selection(cases)),
        ("the shuffle seed", first.get("seed"), seed),
    ):
        if then != now:
            raise RefereeError(f"{what} differ from the run's; no receipt was written")
    settled: list[Row] = [e for e in entries if e["event"] == JOURNAL_SETTLED]
    order: list[int] = case_order(len(cases), seed)
    if len(settled) != len(cases):
        raise RefereeError(f"{path} settled {len(settled)} of {len(cases)} cases")
    client: Client = Client()
    judged: list[Judged] = []
    since: str | None = None
    for index, entry in zip(order, settled, strict=True):
        if entry["case_id"] != cases[index]["case_id"] or "answer" not in entry:
            raise RefereeError(f"{path} settled {entry['case_id']} out of order or without answer")
        case, found, version, at = judged_from(cases[index], entry)
        if isinstance(found, Failure):
            again: Observation | Failure = judge.observation_from(found)
            if isinstance(again, Observation) and not again.statement_id:
                identifier: str = recorded_statement(
                    client=client, failure=found, since=since, until=at
                )
                again = judge.observation_from(replace(found, statement_id=identifier))
            found = again
        judged.append((case, found, version, at))
        since = at
    run: Run = Run(
        run_id=run_id, schema_stem=str(first["schema_stem"]), out=out, private=private, seed=seed
    )
    versions: Versions = Versions(client)
    versions.seen |= {canonical(version) for _, _, version, _ in judged}
    report: Row = finish(client=client, versions=versions, judged=judged, run=run)
    return {**report, "judged_by": first.get("referee"), "phases_by": referee_source()}


def recorded_statement(*, client: Client, failure: Failure, since: str | None, until: str) -> str:
    """The statement a journaled failure came from: the one failed statement on the warehouse
    between the previous case's settling and this case's, whose error message is the recorded one.
    Only one case runs at a time, so its window holds its own statements alone."""
    end: int = millis(until)
    start: int = millis(since) if since is not None else end - 2 * int(CASE_SECONDS * 1000)
    query: str = urllib.parse.urlencode(
        {
            "filter_by.query_start_time_range.start_time_ms": start,
            "filter_by.query_start_time_range.end_time_ms": end,
            "filter_by.warehouse_ids": client.warehouse,
            "max_results": HISTORY_PAGE,
        }
    )
    reply: Row = client._call(method="GET", path=f"{HISTORY_PATH}?{query}")
    if reply.get("has_next_page"):
        raise RefereeError(f"more than {HISTORY_PAGE} statements ran between {since} and {until}")
    failed: list[str] = [
        str(row["query_id"])
        for row in reply.get("res", [])  # type: ignore[union-attr]
        if row.get("status") == HISTORY_FAILED
    ]
    matching: list[str] = [
        identifier
        for identifier in failed
        if dict(dict(client._call(method="GET", path=f"{STATEMENTS_PATH}/{identifier}"))["status"])
        .get("error", {})
        .get("message")  # type: ignore[union-attr]
        == failure.message
    ]
    if len(matching) != 1:
        raise RefereeError(
            f"{len(matching)} failed statements between {since} and {until} carry the recorded "
            f"message {failure.message.strip()[:120]!r}; no receipt was written"
        )
    return matching[0]


def millis(instant: str) -> int:
    """An RFC 3339 instant in milliseconds since the epoch."""
    return int(datetime.fromisoformat(instant).timestamp() * 1000)
