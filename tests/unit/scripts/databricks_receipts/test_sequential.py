"""The sequential phase restores the account state after each case or stops with no receipt."""

from __future__ import annotations

import json
import urllib.request
from pathlib import Path
from typing import cast

import pytest

from scripts.databricks_receipts._helpers.receipts import digest
from scripts.databricks_receipts._helpers.run import undecided_reason
from scripts.databricks_receipts._helpers.sequential import (
    record_sequential,
    refinish_sequential,
    resume_sequential,
)
from scripts.databricks_receipts.classes import client as client_module
from scripts.databricks_receipts.classes.client import Client
from scripts.databricks_receipts.constants import REFEREE_CLASS, REPLY_LOST
from scripts.databricks_receipts.exceptions import RefereeError, ReplyLost, TokenRejected
from scripts.databricks_receipts.main.run import run_referee
from scripts.databricks_receipts.models import Failure
from scripts.databricks_receipts.types import Row
from tests.unit.scripts.databricks_receipts._test_types import (
    CheckTestCase,
    CleanupTestCase,
    DeadlineTestCase,
    ErrorTestCase,
    HistoryTestCase,
    LegacySetupTestCase,
    PolicyAnswerTestCase,
    PolicyRefusalTestCase,
    PolicySweepTestCase,
    RecoverTestCase,
    RefinishTestCase,
    ResendTestCase,
    ResumeRefusalTestCase,
    ResumeTestCase,
    SequentialHeldTestCase,
    StopTestCase,
    TokenTestCase,
    UndecidedTestCase,
)
from tests.unit.scripts.databricks_receipts.helpers import (
    GROUPS,
    POLICY_DROPS,
    SWEEP,
    UC,
    USERS_MISSING,
    FakeClient,
    Frozen,
    adds,
    case,
    fail_on,
    failed,
    frozen_run,
    receipts_in,
    removes,
    stop_report,
    succeeded,
    unmapped_run,
    use,
)

UNSUPPORTED_ON_METASTORE: str = (
    "The securable `m` of type METASTORE cannot be found, and METASTORE is not supported in ABAC"
    " policies. Please verify both the securable type and name for correctness and try again."
)
UNDECIDED: str = "statement s9 was still running after 300 s; cancelled, so its case is undecided"
LEGACY_SETUP: list[str] = ["CREATE TABLE t1 ( c1 INT )", "CREATE TABLE t2 ( c1 INT )"]


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


@pytest.mark.parametrize(
    "test_case",
    (
        CleanupTestCase(
            description="a case that creates catalog v1, then a case that changes nothing",
            sql="CREATE CATALOG v1",
            expected_receipts=2,
            expected_phases=[
                ("db.ddl.create-catalog.k1", "accepted"),
                ("db.query.select.k2", "accepted"),
            ],
            expected_cleanup=[
                {
                    "kind": "catalog",
                    "name": "v1",
                    "statement": "DROP CATALOG IF EXISTS `v1` CASCADE",
                }
            ],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_case_that_creates_catalog_v1_when_run_sequentially_then_it_is_dropped_and_receipted(
    test_case: CleanupTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
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
        cases=[case(test_case.sql), case("SELECT 1", "db.query.select.k2")],
        out=out,
        private=private,
        seed=7,
    )

    receipts: list[Row] = receipts_in(out)
    cleanup: Row = json.loads(next(private.glob("*.sequential-cleanup.json")).read_text())
    final: Row = json.loads(next(private.glob("*.sequential-diff.json")).read_text())
    assert summary["receipts"] == test_case.expected_receipts
    assert sorted((r["case_id"], r["phase"]) for r in receipts) == test_case.expected_phases
    assert cleanup == {
        "db.ddl.create-catalog.k1": test_case.expected_cleanup,
        "db.query.select.k2": [],
        "final metastore policy sweep": SWEEP,
    }
    assert final == {"added": [], "removed": [], "changed": []}
    assert "DROP CATALOG IF EXISTS `v1` CASCADE" in fake.statements
    assert not list(private.glob("*.sequential-stop.json"))


@pytest.mark.parametrize(
    "test_case",
    (
        StopTestCase(
            description="a comment set on the workspace catalog",
            sql="COMMENT ON CATALOG workspace IS 'x'",
            expected_error="removed or changed an object that existed before it",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_case_that_changes_a_pre_existing_object_when_run_sequentially_then_the_run_stops(
    test_case: StopTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    def comment(fake: FakeClient) -> None:
        fake.objects(f"{UC}/catalogs")[2]["comment"] = "x"

    use(monkeypatch, FakeClient({test_case.sql: comment}))
    out: Path = tmp_path / "out"

    with pytest.raises(RefereeError, match=test_case.expected_error):
        record_sequential(
            cases=[case(test_case.sql)],
            out=out,
            private=tmp_path / "private",
            seed=None,
        )

    report: Row = stop_report(tmp_path / "private")
    changed: list[Row] = cast(list[Row], cast(Row, report["diff"])["changed"])
    assert not out.exists()
    assert (report["case_id"], report["sql"]) == (
        "db.ddl.create-catalog.k1",
        "COMMENT ON CATALOG workspace IS 'x'",
    )
    assert changed[0]["name"] == "workspace"
    assert report["dropped"] == []


@pytest.mark.parametrize(
    "test_case",
    (
        StopTestCase(
            description="a script that creates a fixture-named and an other catalog",
            sql="BEGIN CREATE CATALOG v1 ; CREATE CATALOG z9 ; END",
            expected_error="no reviewed fixture name or drop",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_added_object_without_a_fixture_name_when_run_sequentially_then_the_run_stops(
    test_case: StopTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake: FakeClient = use(
        monkeypatch,
        FakeClient({test_case.sql: adds(f"{UC}/catalogs", {"name": "v1"}, {"name": "z9"})}),
    )
    out: Path = tmp_path / "out"

    with pytest.raises(RefereeError, match=test_case.expected_error):
        record_sequential(
            cases=[case(test_case.sql)],
            out=out,
            private=tmp_path / "private",
            seed=None,
        )

    report: Row = stop_report(tmp_path / "private")
    added: list[Row] = cast(list[Row], cast(Row, report["diff"])["added"])
    assert not out.exists()
    assert [a["name"] for a in added] == ["v1", "z9"]
    assert report["dropped"] == []
    assert not any(sql.startswith("DROP CATALOG") for sql in fake.statements)


@pytest.mark.parametrize(
    "test_case",
    (
        StopTestCase(
            description="a catalog drop the warehouse refuses",
            sql="CREATE CATALOG v1",
            expected_error="a cleanup statement failed",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_cleanup_statement_that_fails_when_run_sequentially_then_the_run_stops(
    test_case: StopTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    use(
        monkeypatch,
        FakeClient(
            {
                test_case.sql: adds(f"{UC}/catalogs", {"name": "v1"}),
                "DROP CATALOG IF EXISTS `v1` CASCADE": lambda _: failed("[PERMISSION_DENIED] x"),
            }
        ),
    )
    out: Path = tmp_path / "out"

    with pytest.raises(RefereeError, match=test_case.expected_error):
        record_sequential(
            cases=[case(test_case.sql)], out=out, private=tmp_path / "private", seed=None
        )

    report: Row = stop_report(tmp_path / "private")
    assert not out.exists()
    assert report["dropped"] == []
    assert "PERMISSION_DENIED" in str(report["reason"])


@pytest.mark.parametrize(
    "test_case",
    (
        SequentialHeldTestCase(
            description="a group created",
            sql="CREATE GROUP g1",
            expected_error="1 cases change account- or catalog-level state",
            expected_deleted=["99"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_held_case_when_run_with_and_without_sequential_then_only_sequential_runs_it(
    test_case: SequentialHeldTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake: FakeClient = use(
        monkeypatch,
        FakeClient(
            {
                test_case.sql: adds(
                    GROUPS, {"id": "99", "displayName": "g1", "members": [{"value": "7"}]}
                )
            }
        ),
    )
    cases: Path = tmp_path / "cases.jsonl"
    cases.write_text(json.dumps(case(test_case.sql, "db.security_admin.k3")) + "\n")
    command: list[str] = ["run", "--cases", str(cases), "--out", str(tmp_path / "out")]

    with pytest.raises(RefereeError, match=test_case.expected_error):
        run_referee([*command, "--private", str(tmp_path / "held")])
    assert not (tmp_path / "out").exists()
    assert fake.statements == []

    assert run_referee([*command, "--private", str(tmp_path / "private"), "--sequential"]) == 0
    assert fake.deleted == test_case.expected_deleted
    assert len(list((tmp_path / "out").glob("*.jsonl"))) == 1


@pytest.mark.parametrize(
    "test_case",
    (
        PolicySweepTestCase(
            description="a policy created on the metastore",
            sql="CREATE POLICY v1 ON METASTORE COMMENT 'x'",
            expected_drops=2,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_metastore_policy_case_when_run_sequentially_then_its_policies_are_dropped_twice(
    test_case: PolicySweepTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake: FakeClient = use(monkeypatch, FakeClient({}))
    private: Path = tmp_path / "private"

    record_sequential(
        cases=[case(test_case.sql), case("SELECT 1", "db.q.k2")],
        out=tmp_path / "out",
        private=private,
        seed=7,
    )

    cleanup: Row = json.loads(next(private.glob("*.sequential-cleanup.json")).read_text())
    assert cleanup["db.ddl.create-catalog.k1"] == SWEEP
    assert cleanup["db.q.k2"] == []
    assert cleanup["final metastore policy sweep"] == SWEEP
    assert (
        list(filter(POLICY_DROPS.__contains__, fake.statements))
        == [*POLICY_DROPS] * test_case.expected_drops
    )


@pytest.mark.parametrize(
    "test_case",
    (
        StopTestCase(
            description="a drop of policy v1 that succeeds",
            sql="DROP POLICY `v1` ON METASTORE",
            expected_error="metastore policy drop",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_metastore_policy_drop_that_succeeds_when_run_sequentially_then_the_run_stops(
    test_case: StopTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    use(monkeypatch, FakeClient({test_case.sql: lambda _: succeeded()}))
    out: Path = tmp_path / "out"

    with pytest.raises(RefereeError, match=test_case.expected_error):
        record_sequential(
            cases=[case("SELECT 1", "db.q.k2")], out=out, private=tmp_path / "private", seed=7
        )

    report: Row = stop_report(tmp_path / "private")
    drops: list[Row] = cast(list[Row], report["policy_drops"])
    assert not out.exists()
    assert report["case_id"] is None
    assert drops[0]["state"] == "SUCCEEDED"


@pytest.mark.parametrize(
    "test_case",
    (
        RecoverTestCase(
            description="a run frozen after its case created catalogs v1 and t2 and a schema",
            expected_status=1,
            expected_unsettled=["db.ddl.create-catalog.k1"],
            expected_left=["t2"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_run_that_froze_mid_case_when_recovered_then_only_what_the_case_names_is_dropped(
    test_case: RecoverTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
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
    dropped: list[Row] = cast(list[Row], report["dropped"])
    left: list[Row] = cast(list[Row], cast(Row, report["left"])["added"])
    assert status == test_case.expected_status
    assert report["unsettled"] == test_case.expected_unsettled
    assert [(d["kind"], d["name"]) for d in dropped] == [
        ("case schema", f"{stem}_1"),
        ("catalog", "v1"),
    ]
    assert [a["name"] for a in left] == test_case.expected_left
    assert "DROP CATALOG IF EXISTS `t2` CASCADE" not in fake.statements


@pytest.mark.parametrize(
    "test_case",
    (
        PolicyAnswerTestCase(
            description="an answer that no ABAC policy can be attached to the metastore",
            message=UNSUPPORTED_ON_METASTORE,
            expected_receipts=1,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_policy_drop_answer_that_none_can_exist_when_swept_then_the_run_passes(
    test_case: PolicyAnswerTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    use(
        monkeypatch,
        FakeClient({"DROP POLICY `v1 Q` ON METASTORE": lambda _: failed(test_case.message)}),
    )

    summary: Row = record_sequential(
        cases=[case("SELECT 1", "db.q.k2")], out=tmp_path / "out", private=tmp_path / "p", seed=7
    )

    assert summary["receipts"] == test_case.expected_receipts


@pytest.mark.parametrize(
    "test_case",
    (
        PolicyRefusalTestCase(
            description="an answer that a catalog securable cannot be found",
            message="The securable `m` of type CATALOG cannot be found.",
            expected_error="metastore policy drop",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_another_policy_drop_answer_when_swept_then_the_run_stops(
    test_case: PolicyRefusalTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    use(
        monkeypatch,
        FakeClient({"DROP POLICY `v1 Q` ON METASTORE": lambda _: failed(test_case.message)}),
    )

    with pytest.raises(RefereeError, match=test_case.expected_error):
        record_sequential(
            cases=[case("SELECT 1", "db.q.k2")],
            out=tmp_path / "out",
            private=tmp_path / "p",
            seed=7,
        )


@pytest.mark.parametrize(
    "test_case",
    (
        ResumeTestCase(
            description="a run frozen in its second of three cases",
            expected_receipts=3,
            expected_events=[
                "baseline",
                "pending",
                "settled",
                "pending",
                "pending",
                "settled",
                "pending",
                "settled",
                "finished",
            ],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_run_killed_mid_case_when_resumed_then_it_finishes_without_judging_settled_cases_again(
    test_case: ResumeTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake, journal, cases = frozen_run(tmp_path, monkeypatch)
    fake.statements.clear()

    report: Row = resume_sequential(
        path=journal, cases=cases, out=tmp_path / "out", private=tmp_path / "private", seed=None
    )

    receipts: list[Row] = receipts_in(tmp_path / "out")
    assert report["receipts"] == test_case.expected_receipts
    assert sorted(str(r["case_id"]) for r in receipts) == [
        "db.ddl.create-catalog.k2",
        "db.q.k1",
        "db.q.k3",
    ]
    assert "SELECT 1" not in fake.statements
    assert fake.statements.count("CREATE CATALOG v1") == 1
    assert [c["name"] for c in fake.objects(f"{UC}/catalogs")] == ["samples", "system", "workspace"]
    events: list[str] = [json.loads(line)["event"] for line in journal.read_text().splitlines()]
    assert events == test_case.expected_events


@pytest.mark.parametrize(
    "test_case",
    (
        ErrorTestCase(
            description="a catalog someone else created while the run was stopped",
            expected_error="differs from the stopped run's baseline",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_the_account_changed_while_stopped_when_resumed_then_it_refuses(
    test_case: ErrorTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake, journal, cases = frozen_run(tmp_path, monkeypatch)
    fake.objects(f"{UC}/catalogs").append({"name": "someone_elses"})

    with pytest.raises(RefereeError, match=test_case.expected_error):
        resume_sequential(
            path=journal, cases=cases, out=tmp_path / "out", private=tmp_path / "private", seed=None
        )

    assert not (tmp_path / "out").exists()


@pytest.mark.parametrize(
    "test_case",
    (
        ResumeRefusalTestCase(
            description="order",
            change="order",
            seed=None,
            expected_error="the cases or their order",
        ),
        ResumeRefusalTestCase(
            description="selection",
            change="selection",
            seed=None,
            expected_error="the cases or their order",
        ),
        ResumeRefusalTestCase(
            description="seed", change="seed", seed=7, expected_error="the shuffle seed"
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_other_cases_or_seed_when_resumed_then_it_refuses(
    test_case: ResumeRefusalTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake, journal, cases = frozen_run(tmp_path, monkeypatch)
    given: list[Row] = {"order": cases[::-1], "selection": cases[:2], "seed": cases}[
        test_case.change
    ]
    fake.statements.clear()

    with pytest.raises(RefereeError, match=test_case.expected_error):
        resume_sequential(
            path=journal,
            cases=given,
            out=tmp_path / "out",
            private=tmp_path / "private",
            seed=test_case.seed,
        )

    assert fake.statements == []


@pytest.mark.parametrize(
    "test_case",
    (
        UndecidedTestCase(
            description="a recipient statement cancelled at the deadline",
            expected_receipts=1,
            expected_undecided=["db.r.k2"],
            expected_reasons={"db.r.k2": "cancelled at the deadline"},
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_case_cancelled_at_the_deadline_when_run_then_it_alone_has_no_receipt(
    test_case: UndecidedTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    def cancelled(_: FakeClient) -> None:
        raise RefereeError(UNDECIDED)

    use(monkeypatch, FakeClient({"CREATE RECIPIENT v1": cancelled}))
    cases: list[Row] = [case("SELECT 1", "db.q.k1"), case("CREATE RECIPIENT v1", "db.r.k2")]

    summary: Row = record_sequential(
        cases=cases, out=tmp_path / "out", private=tmp_path / "private", seed=None
    )

    assert (summary["receipts"], summary["undecided"]) == (
        test_case.expected_receipts,
        test_case.expected_undecided,
    )
    assert summary["undecided_reasons"] == test_case.expected_reasons


@pytest.mark.parametrize(
    "test_case",
    (
        StopTestCase(
            description="a recipient statement that ended CANCELED",
            sql="CREATE RECIPIENT v1",
            expected_error="no reviewed row maps",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_another_referee_failure_when_run_then_no_receipt_is_written(
    test_case: StopTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    def ended(_: FakeClient) -> None:
        raise RefereeError("statement ended CANCELED: 'CREATE RECIPIENT v1'")

    use(monkeypatch, FakeClient({test_case.sql: ended}))

    with pytest.raises(RefereeError, match=test_case.expected_error):
        record_sequential(
            cases=[case(test_case.sql, "db.r.k2")],
            out=tmp_path / "out",
            private=tmp_path / "private",
            seed=None,
        )

    assert not (tmp_path / "out").exists()


@pytest.mark.parametrize(
    "test_case",
    (
        CheckTestCase(
            description="the second check fails",
            failing_checks=frozenset({2}),
            expected_error="stopped before db.q.k2, which has not started",
            expected_events=["baseline", "pending", "settled"],
            expected_receipts=2,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_check_that_fails_before_a_case_when_resumed_then_that_case_runs_once(
    test_case: CheckTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake: FakeClient = use(monkeypatch, FakeClient({}))
    fake.failing_checks = set(test_case.failing_checks)
    cases: list[Row] = [case("SELECT 1", "db.q.k1"), case("SELECT 2", "db.q.k2")]
    private: Path = tmp_path / "private"

    with pytest.raises(RefereeError, match=test_case.expected_error):
        record_sequential(cases=cases, out=tmp_path / "out", private=private, seed=None)
    journal: Path = next(private.glob("*.sequential-journal.jsonl"))
    events: list[str] = [json.loads(line)["event"] for line in journal.read_text().splitlines()]
    report: Row = resume_sequential(
        path=journal, cases=cases, out=tmp_path / "out", private=private, seed=None
    )

    assert events == test_case.expected_events
    assert report["receipts"] == test_case.expected_receipts
    assert fake.statements.count("SELECT 2") == 1


@pytest.mark.parametrize(
    "test_case",
    (
        RefinishTestCase(
            description="a member user that does not exist, now reviewed",
            expected_receipts=2,
            expected_phase="semantic_error",
            expected_query_id="h-case",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_failure_a_row_now_maps_when_refinished_then_its_statement_is_found_and_receipted(
    test_case: RefinishTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
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

    receipts: dict[str, Row] = {str(r["case_id"]): r for r in receipts_in(tmp_path / "out")}
    assert report["receipts"] == test_case.expected_receipts
    assert receipts["db.g.k2"]["phase"] == test_case.expected_phase
    assert receipts["db.g.k2"]["query_id_sha256"] == digest(test_case.expected_query_id)
    assert fake.statements == []


@pytest.mark.parametrize(
    "test_case",
    (
        HistoryTestCase(
            description="0",
            messages=("other", "other"),
            expected_error="0 failed statements",
        ),
        HistoryTestCase(
            description="2",
            messages=(USERS_MISSING, USERS_MISSING),
            expected_error="2 failed statements",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_no_single_recorded_statement_when_refinished_then_no_receipt_is_written(
    test_case: HistoryTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fake, journal, cases = unmapped_run(tmp_path, monkeypatch)
    fake.history = [{"query_id": f"h{i}", "status": "FAILED"} for i in range(2)]
    fake.messages = dict(zip(("h0", "h1"), test_case.messages, strict=True))

    with pytest.raises(RefereeError, match=test_case.expected_error):
        refinish_sequential(
            path=journal, cases=cases, out=tmp_path / "out", private=tmp_path / "private", seed=None
        )

    assert not (tmp_path / "out").exists()


@pytest.mark.parametrize(
    "test_case",
    (
        TokenTestCase(
            description="the first request's token rejected",
            expected_reply={"ok": True},
            expected_sent=["Bearer t1", "Bearer t2"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_rejected_token_when_calling_then_a_new_token_is_asked_for_once(
    test_case: TokenTestCase, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("DATABRICKS_HOST", "https://example.com")
    monkeypatch.setenv("DATABRICKS_WAREHOUSE_ID", "w1")
    client: Client = Client()
    tokens: list[str] = []
    sent: list[str] = []

    def token() -> str:
        tokens.append("t")
        return f"t{len(tokens)}"

    def send(*, request: urllib.request.Request, once: bool = False) -> Row:
        sent.append(str(request.get_header("Authorization")))
        fail_on(
            turn=len(sent),
            turns={1},
            failure=TokenRejected("HTTP 403 from the warehouse API: Invalid Token"),
        )
        return {"ok": True}

    monkeypatch.setattr(client, "token", token)
    monkeypatch.setattr(client, "_send", send)

    assert client._call(method="GET", path="/x") == test_case.expected_reply
    assert sent == test_case.expected_sent


@pytest.mark.parametrize(
    "test_case",
    (
        ResendTestCase(
            description="the first reply of each request lost",
            expected_statement=["POST"],
            expected_check=["GET", "GET"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_reply_that_never_arrives_when_calling_then_only_a_statement_is_not_sent_again(
    test_case: ResendTestCase, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("DATABRICKS_HOST", "https://example.com")
    monkeypatch.setenv("DATABRICKS_WAREHOUSE_ID", "w1")
    client: Client = Client()
    monkeypatch.setattr(client, "token", lambda: "t1")
    monkeypatch.setattr(client_module.time, "sleep", lambda _: None)
    sent: list[str] = []

    def urlopen(request: urllib.request.Request, timeout: float) -> Reply:
        sent.append(str(request.get_method()))
        fail_on(turn=len(sent), turns={1}, failure=TimeoutError("The read operation timed out"))
        return Reply(b'{"state": "RUNNING"}')

    monkeypatch.setattr(urllib.request, "urlopen", urlopen)

    with pytest.raises(ReplyLost, match=f"^{REPLY_LOST}"):
        client.execute(sql="BEGIN CREATE TABLE t1 ( c1 INT ) ; END", schema="s1")
    assert sent == test_case.expected_statement

    sent.clear()
    client.check()
    assert sent == test_case.expected_check


@pytest.mark.parametrize(
    "test_case",
    (
        UndecidedTestCase(
            description="one setup whose reply was lost and one the warehouse refused",
            expected_receipts=1,
            expected_undecided=["db.q.k2", "db.q.k3"],
            expected_reasons={
                "db.q.k2": "reply lost",
                "db.q.k3": "setup failed: TABLE_OR_VIEW_ALREADY_EXISTS",
            },
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_setup_that_failed_when_run_then_its_case_alone_is_undecided(
    test_case: UndecidedTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
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

    assert (summary["receipts"], summary["undecided"]) == (
        test_case.expected_receipts,
        test_case.expected_undecided,
    )
    assert summary["undecided_reasons"] == test_case.expected_reasons
    assert "SELECT 2" not in fake.statements
    assert "SELECT 3" not in fake.statements


@pytest.mark.parametrize(
    "test_case",
    (
        StopTestCase(
            description="a setup statement the API never answered",
            sql="CREATE TABLE t2 ( c1 INT )",
            expected_error="no reviewed row maps",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_setup_the_api_never_answered_when_run_then_no_receipt_is_written(
    test_case: StopTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    def unreachable(_: FakeClient) -> None:
        raise RefereeError("the warehouse API is unreachable: timed out")

    use(monkeypatch, FakeClient({test_case.sql: unreachable}))

    with pytest.raises(RefereeError, match=test_case.expected_error):
        record_sequential(
            cases=[{**case("SELECT 2", "db.q.k2"), "setup": [test_case.sql]}],
            out=tmp_path / "out",
            private=tmp_path / "private",
            seed=None,
        )

    assert not (tmp_path / "out").exists()


@pytest.mark.parametrize(
    "test_case",
    (
        LegacySetupTestCase(
            description="the case's own setup",
            statement="BEGIN CREATE TABLE t1 ( c1 INT ) ; CREATE TABLE t2 ( c1 INT ) ; END",
            expected_undecided=True,
        ),
        LegacySetupTestCase(
            description="the case schema's drop",
            statement="DROP SCHEMA IF EXISTS workspace.s1 CASCADE",
            expected_undecided=False,
        ),
        LegacySetupTestCase(
            description="another setup",
            statement="BEGIN CREATE TABLE t9 ( c1 INT ) ; END",
            expected_undecided=False,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_journal_failure_from_before_setup_was_marked_when_judged_then_only_setup_counts(
    test_case: LegacySetupTestCase,
) -> None:
    message: str = f"{test_case.statement!r} failed: {{'error_code': 'BAD_REQUEST'}}"
    failure: Failure = Failure(
        error_class=REFEREE_CLASS, sqlstate="", message=message, parser=False
    )

    reason: str | None = undecided_reason(
        case={**case("SELECT 1"), "setup": LEGACY_SETUP}, failure=failure
    )
    assert (reason is not None) is test_case.expected_undecided


@pytest.mark.parametrize(
    "test_case",
    (
        DeadlineTestCase(
            description="a 900 s deadline",
            seconds="900",
            expected_error=r"^statement s9 was still running after 900 s;",
            expected_calls=["POST statements", "GET s9", "GET s9", "POST cancel"],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_longer_case_deadline_when_a_statement_runs_on_then_it_is_cancelled_after_it(
    test_case: DeadlineTestCase, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("DATABRICKS_HOST", "https://example.com")
    monkeypatch.setenv("DATABRICKS_WAREHOUSE_ID", "w1")
    monkeypatch.setenv("DATABRICKS_REFEREE_CASE_SECONDS", test_case.seconds)
    client: Client = Client()
    clock: list[float] = [0.0, 301.0, 899.0, 901.0]
    monkeypatch.setattr(client_module.time, "monotonic", lambda: clock.pop(0))
    monkeypatch.setattr(client_module.time, "sleep", lambda _: None)
    calls: list[str] = []

    def call(*, method: str, path: str, body: Row | None = None, once: bool = False) -> Row:
        calls.append(f"{method} {path.rsplit('/', 1)[-1]}")
        return {"statement_id": "s9", "status": {"state": "RUNNING"}}

    monkeypatch.setattr(client, "_call", call)

    with pytest.raises(RefereeError, match=test_case.expected_error):
        client.execute(sql="CREATE RECIPIENT v1 PROPERTIES ( v1 )", schema=None)
    assert calls == test_case.expected_calls
