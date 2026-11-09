"""A fake workspace for the sequential phase, its replies and the runs the tests stop part way."""

from __future__ import annotations

import json
import urllib.parse
from collections.abc import Callable
from pathlib import Path
from typing import cast

import pytest

from scripts.databricks_receipts._helpers import judge, sequential
from scripts.databricks_receipts._helpers.sequential import record_sequential
from scripts.databricks_receipts.classes.client import Client
from scripts.databricks_receipts.exceptions import RefereeError
from scripts.databricks_receipts.types import Row

UC: str = "/api/2.1/unity-catalog"
GROUPS: str = "/api/2.0/preview/scim/v2/Groups"
METASTORE: str = "0f0e0d0c-0000-4000-8000-000000000001"
LIST_PAGE: int = 2
NOT_ADMIN: str = "PERMISSION_DENIED: User is not a metastore admin for Metastore 'm'."
POLICY_DROPS: tuple[str, ...] = (
    "DROP POLICY `v1` ON METASTORE",
    "DROP POLICY `v1 Q` ON METASTORE",
    "DROP POLICY `SELECT` ON METASTORE",
)
USERS_MISSING: str = "Users do not exist: v1"
SWEEP: list[Row] = [
    {"statement": sql, "state": "FAILED", "message": NOT_ADMIN} for sql in POLICY_DROPS
]

type Effect = Callable[["FakeClient"], Row | None]
type Route = Callable[..., Row]


class Frozen(Exception):
    """The orb pausing in the middle of a case: nothing after it runs."""


def succeeded(rows: list[list[str]] | None = None) -> Row:
    """A SUCCEEDED reply with the given result rows."""
    return {"statement_id": "q1", "status": {"state": "SUCCEEDED"}, "result": {"data_array": rows}}


def failed(message: str) -> Row:
    """A FAILED reply with the given message and no SQLSTATE."""
    return {"statement_id": "q1", "status": {"state": "FAILED", "error": {"message": message}}}


def refused(*, message: str, sqlstate: str) -> Row:
    """A FAILED reply of the Statement Execution API."""
    return {
        "statement_id": "s1",
        "status": {"state": "FAILED", "sql_state": sqlstate, "error": {"message": message}},
    }


def fail_on(*, turn: int, turns: set[int], failure: Exception) -> None:
    """Raise the failure when this turn is one of the failing turns."""
    for _ in turns.intersection({turn}):
        raise failure


def _not_admin(_: FakeClient) -> Row:
    return failed(NOT_ADMIN)


def _no_effect(_: FakeClient) -> None:
    return None


class FakeClient(Client):
    """A workspace with a few objects, listed two a page; a test gives each statement's effect."""

    def __init__(self, effects: dict[str, Effect]) -> None:
        self.effects: dict[str, Effect] = {**dict.fromkeys(POLICY_DROPS, _not_admin), **effects}
        self.warehouse: str = "w1"
        self.statements: list[str] = []
        self.history: list[Row] = []
        self.messages: dict[str, str] = {}
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
        """The objects one list holds."""
        return self.lists[path][1]

    def _call(self, *, method: str, path: str, body: Row | None = None, once: bool = False) -> Row:
        base, _, query = path.partition("?")
        asked: dict[str, str] = dict(urllib.parse.parse_qsl(query))
        routes: dict[str, Route] = {"DELETE": self._delete_group, "GET": self._get}
        return routes[method](base=base, asked=asked)

    def _delete_group(self, *, base: str, asked: dict[str, str]) -> Row:
        group: str = base.removeprefix(f"{GROUPS}/")
        self.deleted.append(group)
        self.lists[GROUPS][1][:] = list(filter(lambda g: g["id"] != group, self.objects(GROUPS)))
        return {}

    def _get(self, *, base: str, asked: dict[str, str]) -> Row:
        routes: tuple[tuple[str, Route], ...] = (
            ("/api/2.0/sql/warehouses/", self._check),
            ("/api/2.0/sql/history/queries", self._history),
            ("/api/2.0/sql/statements/", self._statement),
            (f"{UC}/current-metastore-assignment", self._metastore),
            (GROUPS, self._scim_page),
            ("", self._token_page),
        )
        route: Route = next(filter(lambda pair: base.startswith(pair[0]), routes))[1]
        return route(base=base, asked=asked)

    def _check(self, *, base: str, asked: dict[str, str]) -> Row:
        self.checks += 1
        fail_on(
            turn=self.checks,
            turns=self.failing_checks,
            failure=RefereeError("HTTP 403 from the warehouse API: Invalid Token"),
        )
        return {"state": "RUNNING"}

    def _history(self, *, base: str, asked: dict[str, str]) -> Row:
        return {"res": self.history, "has_next_page": False}

    def _statement(self, *, base: str, asked: dict[str, str]) -> Row:
        identifier: str = base.rsplit("/", 1)[1]
        return {"status": {"state": "FAILED", "error": {"message": self.messages[identifier]}}}

    def _metastore(self, *, base: str, asked: dict[str, str]) -> Row:
        return {"metastore_id": METASTORE, "workspace_id": 1}

    def _scim_page(self, *, base: str, asked: dict[str, str]) -> Row:
        key, items = self.lists[base]
        start: int = int(asked["startIndex"])
        page: list[Row] = items[start - 1 : start - 1 + min(LIST_PAGE, int(asked["count"]))]
        return {"totalResults": len(items), "startIndex": start, key: page}

    def _token_page(self, *, base: str, asked: dict[str, str]) -> Row:
        key, items = self.lists[base]
        offset: int = int(asked.get("page_token", "0"))
        reply: Row = {key: items[offset : offset + LIST_PAGE]}
        more: dict[bool, Row] = {True: {"next_page_token": str(offset + LIST_PAGE)}, False: {}}
        reply.update(more[offset + LIST_PAGE < len(items)])
        return reply

    def execute(self, *, sql: str, schema: str | None) -> Row:
        exact: dict[str, Route] = {
            "SELECT current_version()": self._version,
            "CREATE TABLE t1 ( c1 INT )": self._setup,
        }
        prefixes: tuple[tuple[str, Route], ...] = (
            ("CREATE SCHEMA workspace.sqlquack_referee_", self._create_schema),
            ("DROP SCHEMA IF EXISTS workspace.sqlquack_referee_", self._drop_schema),
            ("SHOW SCHEMAS IN workspace LIKE", self._show_schemas),
            ("", self._case),
        )
        route: Route = exact.get(sql) or next(filter(lambda p: sql.startswith(p[0]), prefixes))[1]
        return route(sql=sql)

    def _version(self, *, sql: str) -> Row:
        return succeeded([[json.dumps({"dbsql_version": "2026.44"})]])

    def _setup(self, *, sql: str) -> Row:
        return succeeded()

    def _create_schema(self, *, sql: str) -> Row:
        self.objects(f"{UC}/schemas").append({"name": sql.removeprefix("CREATE SCHEMA workspace.")})
        return succeeded()

    def _drop_schema(self, *, sql: str) -> Row:
        schemas: list[Row] = self.objects(f"{UC}/schemas")
        name: str = sql.split(".")[1].removesuffix(" CASCADE")
        schemas[:] = list(filter(lambda s: s["name"] != name, schemas))
        return succeeded()

    def _show_schemas(self, *, sql: str) -> Row:
        names: list[str] = [str(s["name"]) for s in self.objects(f"{UC}/schemas")]
        return succeeded([[name] for name in filter(_is_referee_schema, names)])

    def _case(self, *, sql: str) -> Row:
        self.statements.append(sql)
        return self.effects.get(sql, _no_effect)(self) or succeeded()


def _is_referee_schema(name: str) -> bool:
    return name.startswith("sqlquack_referee_")


def case(sql: str, case_id: str = "db.ddl.create-catalog.k1") -> Row:
    """A case of the given text, with one setup statement."""
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
    """An effect that adds the objects to one list."""

    def effect(fake: FakeClient) -> None:
        fake.objects(path).extend(items)

    return effect


def removes(path: str, name: str) -> Effect:
    """An effect that removes the named object from one list."""

    def effect(fake: FakeClient) -> None:
        fake.objects(path)[:] = list(filter(lambda o: o.get("name") != name, fake.objects(path)))

    return effect


def use(monkeypatch: pytest.MonkeyPatch, fake: FakeClient) -> FakeClient:
    """The fake, as every client the sequential phase makes."""
    monkeypatch.setattr(sequential, "Client", lambda: fake)
    return fake


def stop_report(private: Path) -> Row:
    """The one stop report a stopped run wrote."""
    reports: list[Path] = list(private.glob("*.sequential-stop.json"))
    assert len(reports) == 1
    return json.loads(reports[0].read_text())


def receipts_in(out: Path) -> list[Row]:
    """Every receipt of every receipt file in the directory."""
    found: list[Row] = []
    for path in out.glob("*.jsonl"):
        found += [json.loads(line) for line in path.read_text().splitlines()]
    return found


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


def _unmapped_failure(line: Row) -> bool:
    return line["event"] == "settled" and cast(Row, line["answer"])["kind"] == "failure"


def unmapped_run(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> tuple[FakeClient, Path, list[Row]]:
    """A finished run whose unmapped CREATE GROUP failure was journaled without its statement id."""
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
    for line in filter(_unmapped_failure, lines):
        cast(Row, cast(Row, line["answer"])["found"]).pop("statement_id")
    journal.write_text("".join(json.dumps(line) + "\n" for line in lines))
    return fake, journal, cases
