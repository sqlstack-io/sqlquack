"""The sequential phase restores the account state after each case or stops with no receipt."""

from __future__ import annotations

import json
import urllib.parse
import urllib.request
from collections.abc import Callable
from pathlib import Path

import pytest

from scripts.databricks_referee._helpers import client as client_module
from scripts.databricks_referee._helpers import judge, sequential
from scripts.databricks_referee._helpers.client import Client
from scripts.databricks_referee._helpers.receipts import digest
from scripts.databricks_referee._helpers.run import undecided_reason
from scripts.databricks_referee._helpers.sequential import (
    record_sequential,
    refinish_sequential,
    resume_sequential,
)
from scripts.databricks_referee.constants import REFEREE_CLASS, REPLY_LOST
from scripts.databricks_referee.exceptions import RefereeError, ReplyLost, TokenRejected
from scripts.databricks_referee.main.run import run_referee
from scripts.databricks_referee.models import Failure
from scripts.databricks_referee.types import Row

UC: str = "/api/2.1/unity-catalog"
GROUPS: str = "/api/2.0/preview/scim/v2/Groups"
METASTORE: str = "0f0e0d0c-0000-4000-8000-000000000001"
# The page length the fake lists use, so that every snapshot reads more than one page.
PAGE: int = 2

type Effect = Callable[["FakeClient"], Row | None]


def succeeded(rows: list[list[str]] | None = None) -> Row:
    return {"statement_id": "q1", "status": {"state": "SUCCEEDED"}, "result": {"data_array": rows}}


def failed(message: str) -> Row:
    return {"statement_id": "q1", "status": {"state": "FAILED", "error": {"message": message}}}


class FakeClient(Client):
    """A workspace with a few pre-existing objects; each statement's effect is given by a test."""

    def __init__(self, effects: dict[str, Effect]) -> None:
        self.effects: dict[str, Effect] = effects
        self.warehouse: str = "w1"
        self.statements: list[str] = []
        # The warehouse's query history and each statement's message, as refinish reads them.
        self.history: list[Row] = []
        self.messages: dict[str, str] = {}
        # Checks before a case that fail, by their turn from 1.
        self.failing_checks: set[int] = set()
        self.checks: int = 0
        self.deleted: list[str] = []
        self.lists: dict[str, tuple[str, list[Row]]] = {
            f"{UC}/catalogs": (
                "catalogs",
                [
                    {"name": "samples", "owner": "System user", "updated_at": 1},
                    {"name": "system", "owner": "System user", "updated_at": 1},
                    {"name": "workspace", "owner": "owner@example.com", "comment": None},
                ],
            ),
            f"{UC}/schemas": (
                "schemas",
                [{"name": "default", "owner": "owner@example.com"}, {"name": "information_schema"}],
            ),
            f"{UC}/connections": ("connections", []),
            f"{UC}/shares": ("shares", []),
            f"{UC}/recipients": ("recipients", []),
            f"{UC}/providers": ("providers", [{"name": "databricks", "owner": "System user"}]),
            f"{UC}/external-locations": ("external_locations", []),
            f"{UC}/storage-credentials": (
                "storage_credentials",
                [{"name": "managed", "owner": "System user"}],
            ),
            f"{UC}/credentials": (
                "credentials",
                [{"name": "managed", "owner": "System user", "purpose": "STORAGE"}],
            ),
            "/api/2.1/tag-policies": (
                "tag_policies",
                [{"tag_key": "class", "values": [{"name": "b"}, {"name": "a"}], "id": "p1"}],
            ),
            GROUPS: (
                "Resources",
                [
                    {"id": "11", "displayName": "admins", "members": [{"value": "7"}]},
                    {"id": "12", "displayName": "users", "members": [{"value": "7"}]},
                    {"id": "13", "displayName": "analysts", "members": []},
                ],
            ),
            f"{UC}/permissions/metastore/{METASTORE}": (
                "privilege_assignments",
                [{"principal": "referee", "privileges": ["CREATE_CATALOG", "CREATE_CONNECTION"]}],
            ),
            f"{UC}/permissions/catalog/workspace": (
                "privilege_assignments",
                [{"principal": "referee", "privileges": ["ALL_PRIVILEGES"]}],
            ),
            f"{UC}/tables": ("tables", []),
            f"{UC}/functions": ("functions", []),
            f"{UC}/volumes": ("volumes", []),
        }

    def objects(self, path: str) -> list[Row]:
        return self.lists[path][1]

    def _call(self, *, method: str, path: str, body: Row | None = None) -> Row:
        base, _, query = path.partition("?")
        asked: dict[str, str] = dict(urllib.parse.parse_qsl(query))
        if method == "DELETE":
            group: str = base.removeprefix(f"{GROUPS}/")
            self.deleted.append(group)
            self.lists[GROUPS][1][:] = [g for g in self.objects(GROUPS) if g["id"] != group]
            return {}
        assert method == "GET", path
        if base.startswith("/api/2.0/sql/warehouses/"):
            self.checks += 1
            if self.checks in self.failing_checks:
                raise RefereeError("HTTP 403 from the warehouse API: Invalid Token")
            return {"state": "RUNNING"}
        if base == "/api/2.0/sql/history/queries":
            return {"res": self.history, "has_next_page": False}
        if base.startswith("/api/2.0/sql/statements/"):
            identifier: str = base.rsplit("/", 1)[1]
            return {"status": {"state": "FAILED", "error": {"message": self.messages[identifier]}}}
        if base == f"{UC}/current-metastore-assignment":
            return {"metastore_id": METASTORE, "workspace_id": 1}
        key, items = self.lists[base]
        if base == GROUPS:
            start: int = int(asked["startIndex"])
            page: list[Row] = items[start - 1 : start - 1 + min(PAGE, int(asked["count"]))]
            return {"totalResults": len(items), "startIndex": start, key: page}
        offset: int = int(asked.get("page_token", "0"))
        reply: Row = {key: items[offset : offset + PAGE]}
        if offset + PAGE < len(items):
            reply["next_page_token"] = str(offset + PAGE)
        return reply

    def execute(self, *, sql: str, schema: str | None) -> Row:
        schemas: list[Row] = self.objects(f"{UC}/schemas")
        if sql == "SELECT current_version()":
            return succeeded([[json.dumps({"dbsql_version": "2026.44"})]])
        if sql.startswith("CREATE SCHEMA workspace.sqlquack_referee_"):
            schemas.append({"name": sql.removeprefix("CREATE SCHEMA workspace.")})
            return succeeded()
        if sql.startswith("DROP SCHEMA IF EXISTS workspace.sqlquack_referee_"):
            name: str = sql.split(".")[1].removesuffix(" CASCADE")
            schemas[:] = [s for s in schemas if s["name"] != name]
            return succeeded()
        if sql.startswith("SHOW SCHEMAS IN workspace LIKE"):
            left: list[list[str]] = [
                [str(s["name"])] for s in schemas if str(s["name"]).startswith("sqlquack_referee_")
            ]
            return succeeded(left)
        if sql == "CREATE TABLE t1 ( c1 INT )":
            return succeeded()
        self.statements.append(sql)
        effect: Effect | None = self.effects.get(sql)
        reply: Row | None = effect(self) if effect is not None else None
        if reply is None and effect is None and sql in POLICY_DROPS:
            return failed(NOT_ADMIN)
        return reply or succeeded()


NOT_ADMIN: str = "PERMISSION_DENIED: User is not a metastore admin for Metastore 'm'."
POLICY_DROPS: tuple[str, ...] = (
    "DROP POLICY `v1` ON METASTORE",
    "DROP POLICY `v1 Q` ON METASTORE",
    "DROP POLICY `SELECT` ON METASTORE",
)
SWEEP: list[Row] = [
    {"statement": sql, "state": "FAILED", "message": NOT_ADMIN} for sql in POLICY_DROPS
]


def case(sql: str, case_id: str = "db.ddl.create-catalog.k1") -> Row:
    return {
        "capture": "execute_autocommit_in_throwaway_database",
        "case_id": case_id,
        "expect": "accept",
        "obligations": [],
        "session": {},
        "session_sha256": "0" * 64,
        "setup": ["CREATE TABLE t1 ( c1 INT )"],
        "setup_sha256": "1" * 64,
        "sql": sql,
        "sql_sha256": "2" * 64,
    }


def adds(path: str, *items: Row) -> Effect:
    def effect(fake: FakeClient) -> None:
        fake.objects(path).extend(items)

    return effect


def removes(path: str, name: str) -> Effect:
    def effect(fake: FakeClient) -> None:
        fake.objects(path)[:] = [o for o in fake.objects(path) if o.get("name") != name]

    return effect


def use(monkeypatch: pytest.MonkeyPatch, fake: FakeClient) -> FakeClient:
    monkeypatch.setattr(sequential, "Client", lambda: fake)
    return fake


def stop_report(private: Path) -> Row:
    reports: list[Path] = list(private.glob("*.sequential-stop.json"))
    assert len(reports) == 1
    return json.loads(reports[0].read_text())


def test_given_a_case_that_creates_catalog_v1_when_run_sequentially_then_it_is_dropped_and_receipted(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake: FakeClient = use(
        monkeypatch,
        FakeClient(
            {
                "CREATE CATALOG v1": adds(f"{UC}/catalogs", {"name": "v1", "owner": "referee"}),
                "DROP CATALOG IF EXISTS `v1` CASCADE": removes(f"{UC}/catalogs", "v1"),
            }
        ),
    )
    out: Path = tmp_path / "out"
    private: Path = tmp_path / "private"

    summary: Row = record_sequential(
        cases=[case("CREATE CATALOG v1"), case("SELECT 1", "db.query.select.k2")],
        out=out,
        private=private,
        seed=7,
    )

    receipts: list[Row] = [
        json.loads(line) for path in out.glob("*.jsonl") for line in path.read_text().splitlines()
    ]
    cleanup: Row = json.loads(next(private.glob("*.sequential-cleanup.json")).read_text())
    final: Row = json.loads(next(private.glob("*.sequential-diff.json")).read_text())
    assert summary["receipts"] == 2
    assert sorted((r["case_id"], r["phase"]) for r in receipts) == [
        ("db.ddl.create-catalog.k1", "accepted"),
        ("db.query.select.k2", "accepted"),
    ]
    assert cleanup == {
        "db.ddl.create-catalog.k1": [
            {"kind": "catalog", "name": "v1", "statement": "DROP CATALOG IF EXISTS `v1` CASCADE"}
        ],
        "db.query.select.k2": [],
        "final metastore policy sweep": SWEEP,
    }
    assert final == {"added": [], "removed": [], "changed": []}
    assert "DROP CATALOG IF EXISTS `v1` CASCADE" in fake.statements
    assert not list(private.glob("*.sequential-stop.json"))


def test_given_a_case_that_changes_a_pre_existing_object_when_run_sequentially_then_the_run_stops(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    def comment(fake: FakeClient) -> None:
        fake.objects(f"{UC}/catalogs")[2]["comment"] = "x"

    use(monkeypatch, FakeClient({"COMMENT ON CATALOG workspace IS 'x'": comment}))
    out: Path = tmp_path / "out"

    with pytest.raises(RefereeError, match="removed or changed an object that existed before it"):
        record_sequential(
            cases=[case("COMMENT ON CATALOG workspace IS 'x'")],
            out=out,
            private=tmp_path / "private",
            seed=None,
        )

    report: Row = stop_report(tmp_path / "private")
    assert not out.exists()
    assert (report["case_id"], report["sql"]) == (
        "db.ddl.create-catalog.k1",
        "COMMENT ON CATALOG workspace IS 'x'",
    )
    assert report["diff"]["changed"][0]["name"] == "workspace"  # type: ignore[index]
    assert report["dropped"] == []


def test_given_an_added_object_without_a_fixture_name_when_run_sequentially_then_the_run_stops(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake: FakeClient = use(
        monkeypatch,
        FakeClient(
            {
                "BEGIN CREATE CATALOG v1 ; CREATE CATALOG z9 ; END": adds(
                    f"{UC}/catalogs", {"name": "v1"}, {"name": "z9"}
                )
            }
        ),
    )
    out: Path = tmp_path / "out"

    with pytest.raises(RefereeError, match="no reviewed fixture name or drop"):
        record_sequential(
            cases=[case("BEGIN CREATE CATALOG v1 ; CREATE CATALOG z9 ; END")],
            out=out,
            private=tmp_path / "private",
            seed=None,
        )

    report: Row = stop_report(tmp_path / "private")
    assert not out.exists()
    assert [a["name"] for a in report["diff"]["added"]] == ["v1", "z9"]  # type: ignore[index]
    assert report["dropped"] == []
    assert not any(sql.startswith("DROP CATALOG") for sql in fake.statements)


def test_given_a_cleanup_statement_that_fails_when_run_sequentially_then_the_run_stops(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    use(
        monkeypatch,
        FakeClient(
            {
                "CREATE CATALOG v1": adds(f"{UC}/catalogs", {"name": "v1"}),
                "DROP CATALOG IF EXISTS `v1` CASCADE": lambda _: failed("[PERMISSION_DENIED] x"),
            }
        ),
    )
    out: Path = tmp_path / "out"

    with pytest.raises(RefereeError, match="a cleanup statement failed"):
        record_sequential(
            cases=[case("CREATE CATALOG v1")], out=out, private=tmp_path / "private", seed=None
        )

    report: Row = stop_report(tmp_path / "private")
    assert not out.exists()
    assert report["dropped"] == []
    assert "PERMISSION_DENIED" in str(report["reason"])


def test_given_a_held_case_when_run_with_and_without_sequential_then_only_sequential_runs_it(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake: FakeClient = use(
        monkeypatch,
        FakeClient(
            {
                "CREATE GROUP g1": adds(
                    GROUPS, {"id": "99", "displayName": "g1", "members": [{"value": "7"}]}
                )
            }
        ),
    )
    cases: Path = tmp_path / "cases.jsonl"
    cases.write_text(json.dumps(case("CREATE GROUP g1", "db.security_admin.k3")) + "\n")
    command: list[str] = ["run", "--cases", str(cases), "--out", str(tmp_path / "out")]

    with pytest.raises(RefereeError, match="1 cases change account- or catalog-level state"):
        run_referee([*command, "--private", str(tmp_path / "held")])
    assert not (tmp_path / "out").exists()
    assert fake.statements == []

    assert run_referee([*command, "--private", str(tmp_path / "private"), "--sequential"]) == 0
    assert fake.deleted == ["99"]
    assert len(list((tmp_path / "out").glob("*.jsonl"))) == 1


def test_given_a_metastore_policy_case_when_run_sequentially_then_its_policies_are_dropped_twice(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake: FakeClient = use(monkeypatch, FakeClient({}))
    private: Path = tmp_path / "private"

    record_sequential(
        cases=[case("CREATE POLICY v1 ON METASTORE COMMENT 'x'"), case("SELECT 1", "db.q.k2")],
        out=tmp_path / "out",
        private=private,
        seed=7,
    )

    cleanup: Row = json.loads(next(private.glob("*.sequential-cleanup.json")).read_text())
    assert cleanup["db.ddl.create-catalog.k1"] == SWEEP
    assert cleanup["db.q.k2"] == []
    assert cleanup["final metastore policy sweep"] == SWEEP
    assert [s for s in fake.statements if s in POLICY_DROPS] == [*POLICY_DROPS, *POLICY_DROPS]


def test_given_a_metastore_policy_drop_that_succeeds_when_run_sequentially_then_the_run_stops(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    use(monkeypatch, FakeClient({"DROP POLICY `v1` ON METASTORE": lambda _: succeeded()}))
    out: Path = tmp_path / "out"

    with pytest.raises(RefereeError, match="metastore policy drop"):
        record_sequential(
            cases=[case("SELECT 1", "db.q.k2")], out=out, private=tmp_path / "private", seed=7
        )

    report: Row = stop_report(tmp_path / "private")
    assert not out.exists()
    assert report["case_id"] is None
    assert report["policy_drops"][0]["state"] == "SUCCEEDED"


class Frozen(Exception):
    """The orb pausing in the middle of a case: nothing after it runs."""


def test_given_a_run_that_froze_mid_case_when_recovered_then_only_what_the_case_names_is_dropped(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    def freeze(fake: FakeClient) -> None:
        fake.objects(f"{UC}/catalogs").extend([{"name": "v1"}, {"name": "t2"}])
        fake.objects(f"{UC}/schemas").append({"name": "sqlquack_referee_left_0"})
        raise Frozen

    fake: FakeClient = use(
        monkeypatch,
        FakeClient(
            {
                "CREATE CATALOG v1": freeze,
                "DROP CATALOG IF EXISTS `v1` CASCADE": removes(f"{UC}/catalogs", "v1"),
            }
        ),
    )
    private: Path = tmp_path / "private"
    with pytest.raises(Frozen):
        record_sequential(
            cases=[case("SELECT 1", "db.q.k2"), case("CREATE CATALOG v1")],
            out=tmp_path / "out",
            private=private,
            seed=None,
        )
    journal: Path = next(private.glob("*.sequential-journal.jsonl"))
    events: list[Row] = [json.loads(line) for line in journal.read_text().splitlines()]
    assert [e["event"] for e in events] == ["baseline", "pending", "settled", "pending"]
    assert events[3]["may_create"] == ["v1"]
    stem: str = str(events[0]["schema_stem"])
    fake.objects(f"{UC}/schemas")[-1]["name"] = f"{stem}_1"

    status: int = run_referee(["recover", "--journal", str(journal), "--private", str(private)])

    report: Row = json.loads(next(private.glob("*.sequential-recover.json")).read_text())
    assert status == 1
    assert report["unsettled"] == ["db.ddl.create-catalog.k1"]
    assert [(d["kind"], d["name"]) for d in report["dropped"]] == [  # type: ignore[union-attr]
        ("case schema", f"{stem}_1"),
        ("catalog", "v1"),
    ]
    assert [a["name"] for a in report["left"]["added"]] == ["t2"]  # type: ignore[index]
    assert "DROP CATALOG IF EXISTS `t2` CASCADE" not in fake.statements


UNSUPPORTED_ON_METASTORE: str = (
    "The securable `m` of type METASTORE cannot be found, and METASTORE is not supported in ABAC"
    " policies. Please verify both the securable type and name for correctness and try again."
)


@pytest.mark.parametrize(
    ("message", "stops"),
    [
        (UNSUPPORTED_ON_METASTORE, False),
        ("The securable `m` of type CATALOG cannot be found.", True),
    ],
)
def test_given_a_policy_drop_answer_when_swept_then_only_one_saying_none_can_exist_passes(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, message: str, stops: bool
) -> None:
    use(monkeypatch, FakeClient({"DROP POLICY `v1 Q` ON METASTORE": lambda _: failed(message)}))
    run = lambda: record_sequential(  # noqa: E731
        cases=[case("SELECT 1", "db.q.k2")], out=tmp_path / "out", private=tmp_path / "p", seed=7
    )

    if stops:
        with pytest.raises(RefereeError, match="metastore policy drop"):
            run()
    else:
        assert run()["receipts"] == 1


def frozen_run(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> tuple[FakeClient, Path, list[Row]]:
    """A run of three cases that froze in the middle of its second, after creating catalog v1."""

    def freeze(fake: FakeClient) -> None:
        fake.objects(f"{UC}/catalogs").append({"name": "v1"})
        fake.effects["CREATE CATALOG v1"] = adds(f"{UC}/catalogs", {"name": "v1"})
        raise Frozen

    fake: FakeClient = use(
        monkeypatch,
        FakeClient(
            {
                "CREATE CATALOG v1": freeze,
                "DROP CATALOG IF EXISTS `v1` CASCADE": removes(f"{UC}/catalogs", "v1"),
            }
        ),
    )
    cases: list[Row] = [
        case("SELECT 1", "db.q.k1"),
        case("CREATE CATALOG v1", "db.ddl.create-catalog.k2"),
        case("SELECT 2", "db.q.k3"),
    ]
    private: Path = tmp_path / "private"
    with pytest.raises(Frozen):
        record_sequential(cases=cases, out=tmp_path / "out", private=private, seed=None)
    return fake, next(private.glob("*.sequential-journal.jsonl")), cases


def test_given_a_run_killed_mid_case_when_resumed_then_it_finishes_without_judging_settled_cases_again(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake, journal, cases = frozen_run(tmp_path, monkeypatch)
    fake.statements.clear()

    report: Row = resume_sequential(
        path=journal, cases=cases, out=tmp_path / "out", private=tmp_path / "private", seed=None
    )

    receipts: list[Row] = [
        json.loads(line)
        for line in next((tmp_path / "out").glob("*.jsonl")).read_text().splitlines()
    ]
    assert report["receipts"] == 3
    assert sorted(r["case_id"] for r in receipts) == [
        "db.ddl.create-catalog.k2",
        "db.q.k1",
        "db.q.k3",
    ]
    assert "SELECT 1" not in fake.statements
    assert fake.statements.count("CREATE CATALOG v1") == 1
    assert [c["name"] for c in fake.objects(f"{UC}/catalogs")] == ["samples", "system", "workspace"]
    events: list[str] = [json.loads(line)["event"] for line in journal.read_text().splitlines()]
    assert events == [
        "baseline",
        "pending",
        "settled",
        "pending",
        "pending",
        "settled",
        "pending",
        "settled",
        "finished",
    ]


def test_given_the_account_changed_while_stopped_when_resumed_then_it_refuses(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake, journal, cases = frozen_run(tmp_path, monkeypatch)
    fake.objects(f"{UC}/catalogs").append({"name": "someone_elses"})

    with pytest.raises(RefereeError, match="differs from the stopped run's baseline"):
        resume_sequential(
            path=journal, cases=cases, out=tmp_path / "out", private=tmp_path / "private", seed=None
        )

    assert not (tmp_path / "out").exists()


@pytest.mark.parametrize(
    ("change", "refusal"),
    [
        ("order", "the cases or their order"),
        ("selection", "the cases or their order"),
        ("seed", "the shuffle seed"),
    ],
)
def test_given_other_cases_or_seed_when_resumed_then_it_refuses(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, change: str, refusal: str
) -> None:
    fake, journal, cases = frozen_run(tmp_path, monkeypatch)
    given: list[Row] = {"order": cases[::-1], "selection": cases[:2], "seed": cases}[change]
    seed: int | None = 7 if change == "seed" else None
    fake.statements.clear()

    with pytest.raises(RefereeError, match=refusal):
        resume_sequential(
            path=journal, cases=given, out=tmp_path / "out", private=tmp_path / "private", seed=seed
        )

    assert fake.statements == []


UNDECIDED: str = "statement s9 was still running after 300 s; cancelled, so its case is undecided"


def test_given_a_case_cancelled_at_the_deadline_when_run_then_it_alone_has_no_receipt(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    def cancelled(_: FakeClient) -> None:
        raise RefereeError(UNDECIDED)

    use(monkeypatch, FakeClient({"CREATE RECIPIENT v1": cancelled}))
    cases: list[Row] = [case("SELECT 1", "db.q.k1"), case("CREATE RECIPIENT v1", "db.r.k2")]

    summary: Row = record_sequential(
        cases=cases, out=tmp_path / "out", private=tmp_path / "private", seed=None
    )

    assert (summary["receipts"], summary["undecided"]) == (1, ["db.r.k2"])
    assert summary["undecided_reasons"] == {"db.r.k2": "cancelled at the deadline"}


def test_given_another_referee_failure_when_run_then_no_receipt_is_written(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    def ended(_: FakeClient) -> None:
        raise RefereeError("statement ended CANCELED: 'CREATE RECIPIENT v1'")

    use(monkeypatch, FakeClient({"CREATE RECIPIENT v1": ended}))

    with pytest.raises(RefereeError, match="no reviewed row maps"):
        record_sequential(
            cases=[case("CREATE RECIPIENT v1", "db.r.k2")],
            out=tmp_path / "out",
            private=tmp_path / "private",
            seed=None,
        )

    assert not (tmp_path / "out").exists()


def test_given_a_check_that_fails_before_a_case_when_resumed_then_that_case_runs_once(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake: FakeClient = use(monkeypatch, FakeClient({}))
    fake.failing_checks = {2}
    cases: list[Row] = [case("SELECT 1", "db.q.k1"), case("SELECT 2", "db.q.k2")]
    private: Path = tmp_path / "private"

    with pytest.raises(RefereeError, match="stopped before db.q.k2, which has not started"):
        record_sequential(cases=cases, out=tmp_path / "out", private=private, seed=None)
    journal: Path = next(private.glob("*.sequential-journal.jsonl"))
    events: list[str] = [json.loads(line)["event"] for line in journal.read_text().splitlines()]
    report: Row = resume_sequential(
        path=journal, cases=cases, out=tmp_path / "out", private=private, seed=None
    )

    assert events == ["baseline", "pending", "settled"]
    assert report["receipts"] == 2
    assert fake.statements.count("SELECT 2") == 1


USERS_MISSING: str = "Users do not exist: v1"


def unmapped_run(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> tuple[FakeClient, Path, list[Row]]:
    """A finished run whose CREATE GROUP failed with a message no row mapped then, journaled
    without its statement id, as journals were before failures kept one."""
    fake: FakeClient = use(
        monkeypatch, FakeClient({"CREATE GROUP v1 WITH USER v1": lambda _: failed(USERS_MISSING)})
    )
    cases: list[Row] = [
        case("SELECT 1", "db.q.k1"),
        case("CREATE GROUP v1 WITH USER v1", "db.g.k2"),
    ]
    private: Path = tmp_path / "private"
    with monkeypatch.context() as unreviewed:
        unreviewed.setattr(judge, "UNCLASSED_LATER", ())
        with pytest.raises(RefereeError, match="no reviewed row maps"):
            record_sequential(cases=cases, out=tmp_path / "out", private=private, seed=None)
    journal: Path = next(private.glob("*.sequential-journal.jsonl"))
    lines: list[Row] = [json.loads(line) for line in journal.read_text().splitlines()]
    for line in lines:
        if line["event"] == "settled" and line["answer"]["kind"] == "failure":
            line["answer"]["found"].pop("statement_id")
    journal.write_text("".join(json.dumps(line) + "\n" for line in lines))
    return fake, journal, cases


def test_given_a_failure_a_row_now_maps_when_refinished_then_its_statement_is_found_and_receipted(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake, journal, cases = unmapped_run(tmp_path, monkeypatch)
    fake.history = [
        {"query_id": "h-ok", "status": "FINISHED"},
        {"query_id": "h-other", "status": "FAILED"},
        {"query_id": "h-case", "status": "FAILED"},
    ]
    fake.messages = {"h-other": "a different refusal", "h-case": USERS_MISSING}
    fake.statements.clear()

    report: Row = refinish_sequential(
        path=journal, cases=cases, out=tmp_path / "out", private=tmp_path / "private", seed=None
    )

    receipts: dict[str, Row] = {
        r["case_id"]: r
        for r in map(json.loads, next((tmp_path / "out").glob("*.jsonl")).read_text().splitlines())
    }
    assert report["receipts"] == 2
    assert receipts["db.g.k2"]["phase"] == "semantic_error"
    assert receipts["db.g.k2"]["query_id_sha256"] == digest("h-case")
    assert fake.statements == []


@pytest.mark.parametrize("matching", [0, 2])
def test_given_no_single_recorded_statement_when_refinished_then_no_receipt_is_written(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, matching: int
) -> None:
    fake, journal, cases = unmapped_run(tmp_path, monkeypatch)
    fake.history = [{"query_id": f"h{i}", "status": "FAILED"} for i in range(2)]
    fake.messages = {f"h{i}": USERS_MISSING if i < matching else "other" for i in range(2)}

    with pytest.raises(RefereeError, match=f"{matching} failed statements"):
        refinish_sequential(
            path=journal, cases=cases, out=tmp_path / "out", private=tmp_path / "private", seed=None
        )

    assert not (tmp_path / "out").exists()


def test_given_a_rejected_token_when_calling_then_a_new_token_is_asked_for_once(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("DATABRICKS_HOST", "https://example.com")
    monkeypatch.setenv("DATABRICKS_WAREHOUSE_ID", "w1")
    client: Client = Client()
    tokens: list[str] = []
    sent: list[str] = []

    def token() -> str:
        tokens.append("t")
        return f"t{len(tokens)}"

    def send(request: urllib.request.Request, *, once: bool = False) -> Row:
        sent.append(str(request.get_header("Authorization")))
        if len(sent) == 1:
            raise TokenRejected("HTTP 403 from the warehouse API: Invalid Token")
        return {"ok": True}

    monkeypatch.setattr(client, "token", token)
    monkeypatch.setattr(client, "_send", send)

    assert client._call(method="GET", path="/x") == {"ok": True}
    assert sent == ["Bearer t1", "Bearer t2"]


class Reply:
    """An HTTP reply as urlopen gives it."""

    def __init__(self, body: bytes) -> None:
        self.body: bytes = body

    def __enter__(self) -> Reply:
        return self

    def __exit__(self, *_: object) -> None:
        return None

    def read(self) -> bytes:
        return self.body


def test_given_a_reply_that_never_arrives_when_calling_then_only_a_statement_is_not_sent_again(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("DATABRICKS_HOST", "https://example.com")
    monkeypatch.setenv("DATABRICKS_WAREHOUSE_ID", "w1")
    client: Client = Client()
    monkeypatch.setattr(client, "token", lambda: "t1")
    monkeypatch.setattr(client_module.time, "sleep", lambda _: None)
    sent: list[str] = []

    def urlopen(request: urllib.request.Request, timeout: float) -> Reply:
        sent.append(str(request.get_method()))
        if len(sent) == 1:
            raise TimeoutError("The read operation timed out")
        return Reply(b'{"state": "RUNNING"}')

    monkeypatch.setattr(urllib.request, "urlopen", urlopen)

    with pytest.raises(ReplyLost, match=f"^{REPLY_LOST}"):
        client.execute(sql="BEGIN CREATE TABLE t1 ( c1 INT ) ; END", schema="s1")
    assert sent == ["POST"]

    sent.clear()
    client.check()
    assert sent == ["GET", "GET"]


def test_given_a_setup_that_failed_when_run_then_its_case_alone_is_undecided(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    def lost(_: FakeClient) -> None:
        raise ReplyLost(f"{REPLY_LOST}: The read operation timed out")

    def exists(_: FakeClient) -> Row:
        return failed("[TABLE_OR_VIEW_ALREADY_EXISTS] Cannot create table or view t2.")

    fake: FakeClient = use(
        monkeypatch,
        FakeClient({"CREATE TABLE t2 ( c1 INT )": lost, "CREATE TABLE t3 ( c1 INT )": exists}),
    )
    cases: list[Row] = [
        case("SELECT 1", "db.q.k1"),
        {**case("SELECT 2", "db.q.k2"), "setup": ["CREATE TABLE t2 ( c1 INT )"]},
        {**case("SELECT 3", "db.q.k3"), "setup": ["CREATE TABLE t3 ( c1 INT )"]},
    ]

    summary: Row = record_sequential(
        cases=cases, out=tmp_path / "out", private=tmp_path / "private", seed=None
    )

    assert (summary["receipts"], summary["undecided"]) == (1, ["db.q.k2", "db.q.k3"])
    assert summary["undecided_reasons"] == {
        "db.q.k2": "reply lost",
        "db.q.k3": "setup failed: TABLE_OR_VIEW_ALREADY_EXISTS",
    }
    assert "SELECT 2" not in fake.statements
    assert "SELECT 3" not in fake.statements


def test_given_a_setup_the_api_never_answered_when_run_then_no_receipt_is_written(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    def unreachable(_: FakeClient) -> None:
        raise RefereeError("the warehouse API is unreachable: timed out")

    use(monkeypatch, FakeClient({"CREATE TABLE t2 ( c1 INT )": unreachable}))

    with pytest.raises(RefereeError, match="no reviewed row maps"):
        record_sequential(
            cases=[{**case("SELECT 2", "db.q.k2"), "setup": ["CREATE TABLE t2 ( c1 INT )"]}],
            out=tmp_path / "out",
            private=tmp_path / "private",
            seed=None,
        )

    assert not (tmp_path / "out").exists()


LEGACY_SETUP: list[str] = ["CREATE TABLE t1 ( c1 INT )", "CREATE TABLE t2 ( c1 INT )"]


@pytest.mark.parametrize(
    ("statement", "expected"),
    [
        ("BEGIN CREATE TABLE t1 ( c1 INT ) ; CREATE TABLE t2 ( c1 INT ) ; END", True),
        ("DROP SCHEMA IF EXISTS workspace.s1 CASCADE", False),
        ("BEGIN CREATE TABLE t9 ( c1 INT ) ; END", False),
    ],
)
def test_given_a_journal_failure_from_before_setup_was_marked_when_judged_then_only_setup_counts(
    statement: str, expected: bool
) -> None:
    message: str = f"{statement!r} failed: {{'error_code': 'BAD_REQUEST'}}"
    failure: Failure = Failure(
        error_class=REFEREE_CLASS, sqlstate="", message=message, parser=False
    )

    reason: str | None = undecided_reason({**case("SELECT 1"), "setup": LEGACY_SETUP}, failure)
    assert (reason is not None) is expected


def test_given_a_longer_case_deadline_when_a_statement_runs_on_then_it_is_cancelled_after_it(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("DATABRICKS_HOST", "https://example.com")
    monkeypatch.setenv("DATABRICKS_WAREHOUSE_ID", "w1")
    monkeypatch.setenv("DATABRICKS_REFEREE_CASE_SECONDS", "900")
    client: Client = Client()
    clock: list[float] = [0.0, 301.0, 899.0, 901.0]
    monkeypatch.setattr(client_module.time, "monotonic", lambda: clock.pop(0))
    monkeypatch.setattr(client_module.time, "sleep", lambda _: None)
    calls: list[str] = []

    def call(*, method: str, path: str, body: Row | None = None, once: bool = False) -> Row:
        calls.append(f"{method} {path.rsplit('/', 1)[-1]}")
        return {"statement_id": "s9", "status": {"state": "RUNNING"}}

    monkeypatch.setattr(client, "_call", call)

    with pytest.raises(RefereeError, match=r"^statement s9 was still running after 900 s;"):
        client.execute(sql="CREATE RECIPIENT v1 PROPERTIES ( v1 )", schema=None)
    assert calls == ["POST statements", "GET s9", "GET s9", "POST cancel"]
