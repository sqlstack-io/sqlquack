"""The sequential phase: cases that change account- or catalog-level state, one at a time."""

from __future__ import annotations

import json
import os
import urllib.parse
from dataclasses import asdict, replace
from datetime import datetime
from pathlib import Path
from typing import Any, NoReturn, cast

from scripts.databricks_referee._helpers import judge
from scripts.databricks_referee._helpers.receipts import canonical, digest
from scripts.databricks_referee._helpers.rows import (
    count_of,
    row_of,
    rows_of,
    snapshot_of,
    texts_of,
)
from scripts.databricks_referee._helpers.run import (
    case_order,
    finish,
    judge_one,
    leftover_schemas,
    new_run,
)
from scripts.databricks_referee.classes.client import Client
from scripts.databricks_referee.classes.versions import Versions
from scripts.databricks_referee.constants import (
    CASE_SECONDS,
    DROP_BY_SCIM,
    DROPS,
    FIXTURE_NAMES,
    GROUPS_PATH,
    HISTORY_FAILED,
    HISTORY_PAGE,
    HISTORY_PATH,
    JOURNAL_ANSWER,
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
    PYCACHE,
    QUERY_MARK,
    SCIM_PAGE,
    SNAPSHOT_KINDS,
    STATE_SUCCEEDED,
    STATEMENT_NAMES,
    STATEMENTS_PATH,
)
from scripts.databricks_referee.exceptions import RefereeError
from scripts.databricks_referee.models import Failure, Observation, Run, SnapshotKind
from scripts.databricks_referee.types import Judged, Row, Snapshot


def with_query(*, path: str, query: str) -> str:
    """The path with one more query parameter."""
    return f"{path}{'&' if QUERY_MARK in path else '?'}{query}"


def listed(*, client: Client, kind: SnapshotKind, path: str) -> list[Row]:
    """Every object of one list, page by page."""
    items: list[Row] = []
    if kind.paging == PAGING_SCIM:
        start: int = 1
        while True:
            reply: Row = client._call(
                method="GET",
                path=with_query(path=path, query=f"startIndex={start}&count={SCIM_PAGE}"),
            )
            page: list[Row] = rows_of(reply.get(kind.items) or [])
            items += page
            start += len(page)
            if not page or start > count_of(reply.get("totalResults") or 0):
                return items
    token: str = ""
    while True:
        query: str = f"{PAGING_TOKEN}={urllib.parse.quote(token, safe='')}"
        reply = client._call(
            method="GET", path=with_query(path=path, query=query) if token else path
        )
        items += rows_of(reply.get(kind.items) or [])
        token = str(reply.get("next_page_token") or "")
        if not token:
            return items


def fingerprint(*, item: Row, fields: tuple[str, ...]) -> Row:
    """The fields of an object a change alters, each list sorted."""
    found: Row = {}
    for field in fields:
        head, _, tail = field.partition(".")
        value: object = item.get(head)
        if tail:
            value = sorted(str(row_of(entry).get(tail)) for entry in rows_of(value or []))
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


def difference(*, before: Snapshot, after: Snapshot) -> Row:
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
    value: object = row_of(added["fields"]).get(variant) if variant else None
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
        group: str = urllib.parse.quote(str(row_of(added["fields"])["id"]), safe="")
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
    """Drop each fixture-named metastore policy, which no snapshot can see, or stop the run."""
    answers: list[Row] = []
    for name in METASTORE_POLICY_NAMES:
        sql: str = METASTORE_POLICY_DROP.format(name=quoted(name))
        status: Row = row_of(client.execute(sql=sql, schema=None)["status"])
        message: str = str(row_of(status.get("error") or {}).get("message", ""))
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
    found: Row = difference(before=baseline, after=snapshot(client))
    added: list[Row] = rows_of(found["added"])
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
    left: Row = difference(before=baseline, after=snapshot(client))
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


def journal(*, path: Path, entry: Row) -> None:
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
    dropped: list[Row] = []
    report: Row = {
        "run_id": run_id,
        "unsettled": [e["case_id"] for e in pending],
        "dropped": dropped,
    }
    if entries[-1]["event"] == JOURNAL_FINISHED:
        report["finished"] = True
    client: Client = Client()
    for schema in leftover_schemas(client=client, stem=str(first["schema_stem"])):
        judge.drop(client=client, schema=schema)
        dropped.append({"kind": "case schema", "name": schema})
    recorded: set[str] = set()
    for e in pending:
        recorded |= set(texts_of(e["may_create"]))
    baseline: Snapshot = snapshot_of(first["baseline"])
    found: Row = difference(before=baseline, after=snapshot(client))
    for entry in rows_of(found["added"]):
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
    report["left"] = difference(before=baseline, after=snapshot(client))
    private.mkdir(parents=True, exist_ok=True)
    (private / f"{run_id}.sequential-recover.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n"
    )
    return report


def referee_source() -> str:
    """The digest of the referee's own source, so a resumed run uses the code it started with."""
    root: Path = Path(__file__).resolve().parents[1]
    files: list[Path] = sorted(p for p in root.rglob("*.py") if PYCACHE not in p.parts)
    return digest(canonical({str(p.relative_to(root)): p.read_text() for p in files}))


def selection(cases: list[Row]) -> str:
    """The digest of the cases as given, in order, each whole."""
    return digest(canonical([digest(canonical(case)) for case in cases]))


def answer(judged: Judged) -> Row:
    """A judged case's answer as a journal line holds it."""
    _, found, version, at = judged
    kind: str = JOURNAL_OBSERVATION if isinstance(found, Observation) else JOURNAL_FAILURE
    return {"kind": kind, "found": asdict(found), "version": version, "recorded_at": at}


def judged_from(*, case: Row, entry: Row) -> Judged:
    """A settled journal line's case answer, as it was judged."""
    found: Row = row_of(entry[JOURNAL_ANSWER])
    model: type[Observation] | type[Failure] = (
        Observation if found["kind"] == JOURNAL_OBSERVATION else Failure
    )
    fields: dict[str, Any] = row_of(found["found"])
    return case, model(**fields), row_of(found["version"]), str(found["recorded_at"])


def judge_remaining(
    *, cases: list[Row], run: Run, baseline: Snapshot, log: Path, judged: list[Judged]
) -> Row:
    """Judge the cases not yet settled, one at a time, then sweep, compare and write receipts."""
    client: Client = Client()
    versions: Versions = Versions(client)
    versions.seen |= {canonical(version) for _, _, version, _ in judged}
    cleanup: dict[str, list[Row]] = {}
    for index in case_order(count=len(cases), seed=run.seed)[len(judged) :]:
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
            path=log,
            entry={
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
            path=log,
            entry={
                "event": JOURNAL_SETTLED,
                "case_id": case["case_id"],
                "dropped": dropped,
                JOURNAL_ANSWER: answer(one),
                "account": JOURNAL_RESTORED,
            },
        )
    cleanup["final metastore policy sweep"] = sweep_metastore_policies(
        client=client, run=run, case=None
    )
    journal(path=log, entry={"event": JOURNAL_FINISHED})
    final: Row = difference(before=baseline, after=snapshot(client))
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
    """Judge the cases one at a time, held cases included, restoring the account after each."""
    run: Run = new_run(out=out, private=private, seed=seed)
    baseline: Snapshot = snapshot(Client())
    named: list[Row] = []
    for kind, objects in baseline.items():
        named += [{"kind": kind, "name": name} for name in objects if name.lower() in FIXTURE_NAMES]
    if named:
        reason: str = "the account already holds objects with fixture names, before any case ran"
        stop(run=run, report={"case_id": None, "sql": None, "reason": reason, "objects": named})
    private.mkdir(parents=True, exist_ok=True)
    log: Path = journal_path(private=private, run_id=run.run_id)
    journal(
        path=log,
        entry={
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
    """Continue a stopped sequential run from its journal, or refuse when anything differs."""
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
    order: list[int] = case_order(count=len(cases), seed=seed)
    judged: list[Judged] = []
    for index, entry in zip(order, settled, strict=False):
        if entry["case_id"] != cases[index]["case_id"] or JOURNAL_ANSWER not in entry:
            raise RefereeError(f"{path} settled {entry['case_id']} out of order or without answer")
        judged.append(judged_from(case=cases[index], entry=entry))
    if len(judged) != len(settled):
        raise RefereeError(f"{path} settled more cases than were selected")
    recovered: Row = recover(path=path, private=private)
    baseline: Snapshot = snapshot_of(first["baseline"])
    found: Row = difference(before=baseline, after=snapshot(Client()))
    if any(row_of(recovered["left"]).values()) or any(found.values()):
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
    """A finished sequential run's receipts, its failures judged again by the current rows."""
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
    order: list[int] = case_order(count=len(cases), seed=seed)
    if len(settled) != len(cases):
        raise RefereeError(f"{path} settled {len(settled)} of {len(cases)} cases")
    client: Client = Client()
    judged: list[Judged] = []
    since: str | None = None
    for index, entry in zip(order, settled, strict=True):
        if entry["case_id"] != cases[index]["case_id"] or JOURNAL_ANSWER not in entry:
            raise RefereeError(f"{path} settled {entry['case_id']} out of order or without answer")
        case, recorded, version, at = judged_from(case=cases[index], entry=entry)
        again: Observation | Failure = judged_again(
            client=client, found=recorded, since=since, until=at
        )
        judged.append((case, again, version, at))
        since = at
    run: Run = Run(
        run_id=run_id, schema_stem=str(first["schema_stem"]), out=out, private=private, seed=seed
    )
    versions: Versions = Versions(client)
    versions.seen |= {canonical(version) for _, _, version, _ in judged}
    report: Row = finish(client=client, versions=versions, judged=judged, run=run)
    return {**report, "judged_by": first.get("referee"), "phases_by": referee_source()}


def judged_again(
    *, client: Client, found: Observation | Failure, since: str | None, until: str
) -> Observation | Failure:
    """A recorded answer as the current reviewed rows judge it."""
    if not isinstance(found, Failure):
        return found
    again: Observation | Failure = judge.observation_from(found)
    if isinstance(again, Observation) and not again.statement_id:
        identifier: str = recorded_statement(client=client, failure=found, since=since, until=until)
        again = judge.observation_from(replace(found, statement_id=identifier))
    return again


def recorded_message(*, client: Client, identifier: str) -> object:
    """The error message of one statement the warehouse ran."""
    reply: Row = client._call(method="GET", path=f"{STATEMENTS_PATH}/{identifier}")
    error: Row = cast(Row, row_of(row_of(reply)["status"]).get("error", {}))
    return error.get("message")


def recorded_statement(*, client: Client, failure: Failure, since: str | None, until: str) -> str:
    """The one failed statement since the previous case settled with the recorded message."""
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
        for row in rows_of(reply.get("res", []))
        if row.get("status") == HISTORY_FAILED
    ]
    matching: list[str] = [
        identifier
        for identifier in failed
        if recorded_message(client=client, identifier=identifier) == failure.message
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
