"""The syntax receipt importer against small, fabricated run directories."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from scripts.syntax_receipts._helpers.hashing import canonical_json
from scripts.syntax_receipts._helpers.receipts import classify, reason_category
from scripts.syntax_receipts._helpers.runs import import_runs
from scripts.syntax_receipts.main.import_runs import run_import
from scripts.syntax_receipts.models import Imported
from scripts.syntax_receipts.types import Row
from tests.unit.scripts.syntax_receipts._test_types import (
    BindingTestCase,
    ClassifyTestCase,
    OverwriteTestCase,
    QueryHistoryReceiptTestCase,
    ReconciledRunTestCase,
    RefusedRunTestCase,
    RepublishTestCase,
    SetupRunTestCase,
    SupersedeTestCase,
)
from tests.unit.scripts.syntax_receipts.helpers import (
    accepted_row,
    error_row,
    import_twice,
    imported_run,
    packet_case,
    removed,
    replaced,
    sha,
    unrecorded_submission,
    write_reconciliation,
    write_run,
)

_GOOD: str = "BEGIN RETURN 1; END;"
_OTHER: str = "BEGIN RETURN 2; END;"
_BROKEN: str = "CREATE mTASK t AS SELECT 1"
_GOOD_SHA: str = hashlib.sha256(_GOOD.encode()).hexdigest()
_OTHER_SHA: str = hashlib.sha256(_OTHER.encode()).hexdigest()
_RECORDED: list[tuple[str, str]] = [("c1", "accepted"), ("c2", "semantic_error")]
_HOLD: str = "reconciliation does not hold: "
_RECONCILED_ROWS: list[dict[str, object]] = [
    {**accepted_row(case_id="c1", sql=_GOOD), "case_schema": "C0000"},
    error_row(case_id="c2", sql=_OTHER, errno=2003, message="SQL compilation error: x"),
]
_RECONCILED_PACKET: list[dict[str, object]] = [
    packet_case(case_id="c1", sql=_GOOD),
    packet_case(case_id="c2", sql=_OTHER),
    packet_case(case_id="c3", sql=_BROKEN),
]
_FIXTURE: list[str] = ['CREATE TABLE "ORDERS" ("ID" INTEGER)']
_DDL_TEMPLATE: list[str] = [
    "CREATE DATABASE <run_database> COMMENT = 'sqlquack ddl sweep <run_nonce>'",
    "CREATE SCHEMA <run_database>.CASES",
    "USE SCHEMA <run_database>.CASES",
    "CREATE SCHEMA <run_database>.<case_schema>",
    "USE SCHEMA <run_database>.<case_schema>",
]
_SYNTAX_ROW: dict[str, object] = error_row(
    case_id="c1", sql=_GOOD, errno=1003, message="SQL compilation error:\nsyntax error"
)
# The connector's own fields for a read timeout: no SQLSTATE class, so the parent code read errno 250003 as an
# execution-time code and imported a runtime_error receipt.
_TIMED_OUT_ROW: dict[str, object] = replaced(
    replaced(
        error_row(case_id="c2", sql=_OTHER, errno=250003, message="Read timed out."),
        ("connector_fields", "case_result"),
        False,
    ),
    ("connector_fields", "sqlstate"),
    "n/a",
)


@pytest.mark.parametrize(
    "test_case",
    (
        ClassifyTestCase(
            description="errno 1003 with the compilation prefix is a syntax error",
            errno=1003,
            message="SQL compilation error:\nsyntax error line 1",
            expected_classification=("syntax_error", "errno 1003 SQL compilation syntax error"),
        ),
        ClassifyTestCase(
            description="another compilation error is semantic",
            errno=2003,
            message="SQL compilation error:\nObject 'T' does not exist",
            expected_classification=(
                "semantic_error",
                "errno 2003 compilation error after parsing",
            ),
        ),
        ClassifyTestCase(
            description="errno 2 Unsupported feature is an unsupported feature",
            errno=2,
            message="Unsupported feature 'MINUS ALL'.",
            expected_classification=("unsupported_feature", "errno 2 Unsupported feature"),
        ),
        ClassifyTestCase(
            description="a 100000-series code is raised while the statement executes",
            errno=100051,
            sqlstate="22012",
            message="Division by zero",
            expected_classification=("runtime_error", "errno 100051 is an execution-time code"),
        ),
        ClassifyTestCase(
            description="a syntax error raised while a block runs is not established",
            errno=1003,
            message="Uncaught exception of type 'EXPRESSION_ERROR' : SQL compilation error",
            expected_classification=None,
        ),
        ClassifyTestCase(
            description="an uncaught exception with another errno ran past compilation",
            errno=100183,
            message="Uncaught exception of type 'STATEMENT_ERROR' on line 5",
            expected_classification=(
                "runtime_error",
                "uncaught exception raised while the block ran",
            ),
        ),
        ClassifyTestCase(
            description="a high errno with SQLSTATE class 42 and no review is not established",
            errno=392032,
            sqlstate="42601",
            message="Streaming alert condition must not contain a show statement",
            expected_classification=None,
        ),
        ClassifyTestCase(
            description="a reviewed high errno with SQLSTATE class 42 is semantic",
            errno=391858,
            sqlstate="42601",
            message="Projection policy must have exactly zero arguments, got 1 arguments.",
            expected_classification=(
                "semantic_error",
                "errno 391858 SQLSTATE 42601, reviewed as semantic",
            ),
        ),
        ClassifyTestCase(
            description="a syntax error without the compilation prefix is not established",
            errno=1003,
            message="syntax error without the compilation prefix",
            expected_classification=None,
        ),
        ClassifyTestCase(
            description="no error number and no message is not established",
            errno=None,
            message="",
            expected_classification=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_error_fields_when_classified_then_only_established_phases_are_named(
    test_case: ClassifyTestCase,
) -> None:
    classified: tuple[str, str] | None = classify(
        errno=test_case.errno, message=test_case.message, sqlstate=test_case.sqlstate
    )

    assert classified == test_case.expected_classification, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        ClassifyTestCase(
            description="a missing named object is a missing-object reason",
            errno=2003,
            message="SQL compilation error:\nSchema 'SALES' does not exist or not authorized.",
            expected_classification=("missing_object", "errno 2003"),
        ),
        ClassifyTestCase(
            description="a duplicated property is an invalid option or value",
            errno=1424,
            message="SQL compilation error:\nduplicate property 'COMMENT';",
            expected_classification=("invalid_option_or_value", "errno 1424"),
        ),
        ClassifyTestCase(
            description="a property of the wrong type is a type reason, not an invalid value",
            errno=1421,
            message="SQL compilation error:\ninvalid type of property '17' for 'ALLOWED_IP_LIST'",
            expected_classification=("type", "errno 1421"),
        ),
        ClassifyTestCase(
            description="an unlisted code whose message names a missing object is missing-object",
            errno=100132,
            message="Execution of multiple statements failed: Table 'T' does not exist.",
            expected_classification=("missing_object", "message names a missing object"),
        ),
        ClassifyTestCase(
            description="an existing object is not a missing one",
            errno=2002,
            message="SQL compilation error:\nObject 'T' already exists.",
            expected_classification=("other", "errno 2002 has no more specific reason"),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_non_syntax_errors_when_categorized_then_snowflake_reason_is_named(
    test_case: ClassifyTestCase,
) -> None:
    category: tuple[str, str] = reason_category(errno=test_case.errno, message=test_case.message)

    assert category == test_case.expected_classification, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        BindingTestCase(
            description="rows with another hash, an unknown id or no phase are refused",
            rows=[
                _SYNTAX_ROW,
                {"id": "c2", "sql_sha256": sha(b"something else"), "outcome": "accepted_executed"},
                accepted_row(case_id="c9", sql=_GOOD),
                error_row(case_id="c3", sql=_OTHER, errno=1003, message="no prefix"),
                replaced(
                    replaced(
                        error_row(
                            case_id="c4", sql=_OTHER, errno=250003, message="Read timed out."
                        ),
                        ("connector_fields", "case_result"),
                        False,
                    ),
                    ("connector_fields", "sqlstate"),
                    "n/a",
                ),
            ],
            packet=[
                packet_case(case_id="c1", sql=_GOOD),
                packet_case(case_id="c2", sql=_OTHER),
                packet_case(case_id="c3", sql=_OTHER),
                packet_case(case_id="c4", sql=_OTHER),
            ],
            expected_receipts=[("c1", "syntax_error")],
            expected_refusals=[
                ("c2", "raw"),
                ("c3", "error"),
                ("c4", "client-side"),
                ("c9", "row"),
            ],
            expected_sql_sha256=sha(_GOOD.encode()),
            expected_raw_record_sha256=sha(json.dumps(_SYNTAX_ROW).encode()),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_bound_rows_when_imported_then_unbindable_rows_are_refused(
    test_case: BindingTestCase, tmp_path: Path
) -> None:
    imported: Imported = imported_run(root=tmp_path, rows=test_case.rows, packet=test_case.packet)

    assert [(r["case_id"], r["phase"]) for r in imported.receipts] == test_case.expected_receipts
    assert (
        sorted((r["row_id"], r["reason"].split()[0]) for r in imported.refusals)
        == test_case.expected_refusals
    )
    assert imported.receipts[0]["sql_sha256"] == test_case.expected_sql_sha256
    assert imported.receipts[0]["raw_record_sha256"] == test_case.expected_raw_record_sha256


@pytest.mark.parametrize(
    "test_case",
    (
        RefusedRunTestCase(
            description="a raw file whose hash differs from the complete event",
            rows=[error_row(case_id="c1", sql=_GOOD, errno=1003, message="x")],
            packet=[packet_case(case_id="c1", sql=_GOOD)],
            raw_hash="0" * 64,
            expected_status="refused",
            expected_reason_fragment="differs from the audit",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_raw_changed_after_audit_when_imported_then_the_whole_run_is_refused(
    test_case: RefusedRunTestCase, tmp_path: Path
) -> None:
    imported: Imported = imported_run(
        root=tmp_path, rows=test_case.rows, packet=test_case.packet, raw_hash=test_case.raw_hash
    )

    assert imported.receipts == [], test_case.description
    assert imported.runs[0]["status"] == test_case.expected_status, test_case.description
    assert test_case.expected_reason_fragment in imported.runs[0]["reason"], test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        OverwriteTestCase(
            description="a second import into the same files",
            rows=[accepted_row(case_id="c1", sql=_GOOD)],
            packet=[packet_case(case_id="c1", sql=_GOOD)],
            expected_error=FileExistsError,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_existing_receipts_when_imported_again_then_nothing_is_overwritten(
    test_case: OverwriteTestCase, tmp_path: Path
) -> None:
    with pytest.raises(test_case.expected_error):
        import_twice(root=tmp_path, rows=test_case.rows, packet=test_case.packet)


@pytest.mark.parametrize(
    "test_case",
    (
        RepublishTestCase(
            description="a semantic error published before the reason category existed",
            rows=[
                error_row(
                    case_id="c1",
                    sql=_GOOD,
                    errno=2003,
                    message="SQL compilation error:\nObject 'T' does not exist",
                )
            ],
            packet=[packet_case(case_id="c1", sql=_GOOD)],
            fields_added_later=("reason_category", "reason_category_basis"),
            expected_new_receipts="",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_receipt_published_before_a_field_was_added_when_imported_again_then_it_is_skipped(
    test_case: RepublishTestCase, tmp_path: Path
) -> None:
    write_run(root=tmp_path, rows=test_case.rows, packet=test_case.packet)
    runs, inputs = str(tmp_path / "runs"), str(tmp_path / "inputs")
    first: Path = tmp_path / "first.jsonl"
    run_import([runs, str(first), str(tmp_path / "first.json"), inputs])
    older: dict[str, object] = json.loads(first.read_text())
    for name in test_case.fields_added_later:
        del older[name]
    first.write_text(json.dumps(older) + "\n")
    second: Path = tmp_path / "second.jsonl"

    run_import(
        [runs, str(second), str(tmp_path / "second.json"), inputs, "--published", str(first)]
    )

    assert second.read_text() == test_case.expected_new_receipts, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        SetupRunTestCase(
            description="an EXPLAIN run in a run database with fixture setup per item",
            rows=[
                {
                    **accepted_row(case_id="c1", sql="SELECT id FROM orders"),
                    "outcome": "compiled",
                    "case_schema": "C0000",
                },
                {
                    **error_row(case_id="c2", sql="SELECT FROM orders", errno=2003, message="x"),
                    "outcome": "setup_failed",
                    "case_schema": "C0001",
                },
            ],
            packet=[
                {**packet_case(case_id="c1", sql="SELECT id FROM orders"), "setup": _FIXTURE},
                {**packet_case(case_id="c2", sql="SELECT FROM orders"), "setup": _FIXTURE},
            ],
            start={
                "run_database": "SQLQUACK_DDL_ABC123",
                "setup": ["CREATE DATABASE SQLQUACK_DDL_ABC123"],
                "method": "EXPLAIN USING TEXT, once",
                "measure": "missing_object_with_setup",
            },
            expected_receipts=[("c1", "accepted", "compile_only_explain")],
            expected_setup=[
                "CREATE DATABASE <run_database>",
                "CREATE SCHEMA <run_database>.<case_schema>",
                "USE SCHEMA <run_database>.<case_schema>",
                *_FIXTURE,
            ],
            expected_refusals=["c2"],
        ),
        SetupRunTestCase(
            description="a packet that repeats the run's template is not counted twice",
            rows=[{**_SYNTAX_ROW, "case_schema": "C0000"}],
            packet=[{**packet_case(case_id="c1", sql=_GOOD), "setup": [*_DDL_TEMPLATE, *_FIXTURE]}],
            start={
                "run_database": "SQLQUACK_DDL_ABC123",
                "setup": [
                    "CREATE DATABASE SQLQUACK_DDL_ABC123 COMMENT = 'sqlquack ddl sweep ABC123'",
                    "CREATE SCHEMA SQLQUACK_DDL_ABC123.CASES",
                    "USE SCHEMA SQLQUACK_DDL_ABC123.CASES",
                ],
            },
            expected_receipts=[("c1", "syntax_error", "execute_ddl_in_run_database")],
            expected_setup=[*_DDL_TEMPLATE, *_FIXTURE],
            expected_refusals=[],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_fixture_setup_per_item_when_imported_then_receipts_name_that_setup(
    test_case: SetupRunTestCase, tmp_path: Path
) -> None:
    write_run(root=tmp_path, rows=test_case.rows, packet=test_case.packet, start=test_case.start)
    out: Path = tmp_path / "out.jsonl"

    run_import(
        [str(tmp_path / "runs"), str(out), str(tmp_path / "out.json"), str(tmp_path / "inputs")]
    )

    receipts: list[Row] = [json.loads(line) for line in out.read_text().splitlines()]
    report: Row = json.loads((tmp_path / "out.json").read_text())
    setups: dict[tuple[str, str], list[str]] = {
        (row["sql_sha256"], row["setup_sha256"]): row["setup"] for row in report["inputs"]
    }
    assert [(r["case_id"], r["phase"], r["capture_method"]) for r in receipts] == (
        test_case.expected_receipts
    )
    receipt_input: tuple[str, str] = (receipts[0]["sql_sha256"], receipts[0]["setup_sha256"])
    assert setups[receipt_input] == test_case.expected_setup
    assert [r["row_id"] for r in report["refusals"]] == test_case.expected_refusals
    assert receipts[0].get("measure") == test_case.start.get("measure")


@pytest.mark.parametrize(
    "test_case",
    (
        BindingTestCase(
            description="a client timeout in a run database is refused, not a runtime error",
            rows=[
                {**_SYNTAX_ROW, "case_schema": "C0000"},
                {**_TIMED_OUT_ROW, "case_schema": "C0001"},
            ],
            packet=[packet_case(case_id="c1", sql=_GOOD), packet_case(case_id="c2", sql=_OTHER)],
            expected_receipts=[("c1", "syntax_error")],
            expected_refusals=[("c2", "client-side")],
            expected_sql_sha256=sha(_GOOD.encode()),
            expected_raw_record_sha256=sha(
                json.dumps({**_SYNTAX_ROW, "case_schema": "C0000"}).encode()
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_client_timeout_in_a_run_database_when_imported_then_it_is_refused(
    test_case: BindingTestCase, tmp_path: Path
) -> None:
    write_run(
        root=tmp_path,
        rows=test_case.rows,
        packet=test_case.packet,
        start={
            "run_database": "SQLQUACK_DDL_ABC123",
            "setup": ["CREATE DATABASE SQLQUACK_DDL_ABC123"],
        },
    )

    imported: Imported = import_runs(runs=tmp_path / "runs", input_dirs=[tmp_path / "inputs"])

    assert [(r["case_id"], r["phase"]) for r in imported.receipts] == test_case.expected_receipts
    assert (
        sorted((r["row_id"], r["reason"].split()[0]) for r in imported.refusals)
        == test_case.expected_refusals
    )
    assert imported.receipts[0]["sql_sha256"] == test_case.expected_sql_sha256
    assert imported.receipts[0]["raw_record_sha256"] == test_case.expected_raw_record_sha256


@pytest.mark.parametrize(
    "test_case",
    (
        SupersedeTestCase(
            description="a receipt published as runtime under the old rule is withdrawn",
            rows=[
                error_row(
                    case_id="c1",
                    sql=_GOOD,
                    errno=1003,
                    message="Uncaught exception of type 'EXPRESSION_ERROR' : SQL compilation error",
                )
            ],
            packet=[packet_case(case_id="c1", sql=_GOOD)],
            published_phase="runtime_error",
            expected_new_receipts=0,
            expected_withdrawn=[("c1", "runtime_error")],
        ),
        SupersedeTestCase(
            description="a receipt whose phase changed is republished as superseding",
            rows=[
                error_row(case_id="c1", sql=_GOOD, errno=2003, message="SQL compilation error: x")
            ],
            packet=[packet_case(case_id="c1", sql=_GOOD)],
            published_phase="runtime_error",
            expected_new_receipts=1,
            expected_withdrawn=[],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_published_phase_changed_when_imported_then_it_is_superseded_explicitly(
    test_case: SupersedeTestCase, tmp_path: Path
) -> None:
    write_run(root=tmp_path, rows=test_case.rows, packet=test_case.packet)
    runs, inputs = str(tmp_path / "runs"), str(tmp_path / "inputs")
    earlier: Path = tmp_path / "earlier.jsonl"
    run_import([runs, str(earlier), str(tmp_path / "earlier.json"), inputs])
    raw: dict[str, object] = json.loads(
        (tmp_path / "runs").joinpath("b5-test-20261103T000000Z", "capture", "raw.jsonl").read_text()
    )
    published: dict[str, object] = {
        "run_id": "b5-test-20261103T000000Z",
        "case_id": raw["id"],
        "raw_record_sha256": sha(json.dumps(raw).encode()),
        "phase": test_case.published_phase,
    }
    earlier.write_text(json.dumps(published) + "\n")
    later: Path = tmp_path / "later.jsonl"

    run_import(
        [runs, str(later), str(tmp_path / "later.json"), inputs, "--published", str(earlier)]
    )

    new: list[Row] = [json.loads(line) for line in later.read_text().splitlines()]
    report: Row = json.loads((tmp_path / "later.json").read_text())
    assert len(new) == test_case.expected_new_receipts, test_case.description
    assert all("supersedes_receipt_sha256" in r for r in new), test_case.description
    assert [(w["case_id"], w["phase"]) for w in report["withdrawn"]] == test_case.expected_withdrawn


@pytest.mark.parametrize(
    "test_case",
    (
        ReconciledRunTestCase(
            description="a stalled run's listed records and its failed query-history submission",
            edit=lambda r: r,
            expected_receipts=[*_RECORDED, ("c3", "syntax_error")],
            expected_refusals=[],
        ),
        ReconciledRunTestCase(
            description="a query-history submission that succeeded",
            edit=lambda r: replaced(
                replaced(
                    replaced(r, ("unrecorded_submission", "execution_status"), "SUCCESS"),
                    ("unrecorded_submission", "errno"),
                    None,
                ),
                ("unrecorded_submission", "error_message"),
                None,
            ),
            expected_receipts=[*_RECORDED, ("c3", "accepted")],
            expected_refusals=[],
        ),
        ReconciledRunTestCase(
            description="a raw record the reconciliation does not list",
            edit=lambda r: replaced(
                replaced(r, ("raw_records", "records"), r["raw_records"]["records"][:1]),
                ("raw_records", "count"),
                1,
            ),
            expected_receipts=[("c1", "accepted"), ("c3", "syntax_error")],
            expected_refusals=[("c2", "raw record is not listed in the run's reconciliation")],
        ),
        ReconciledRunTestCase(
            description="a raw record whose hash differs from the reconciliation",
            edit=lambda r: replaced(r, ("raw_records", "records", 0, "sha256"), "0" * 64),
            expected_receipts=[("c2", "semantic_error"), ("c3", "syntax_error")],
            expected_refusals=[("c1", "raw record hash differs from the run's reconciliation")],
        ),
        ReconciledRunTestCase(
            description="a listed record the run never recorded",
            edit=lambda r: replaced(
                replaced(
                    r,
                    ("raw_records", "records"),
                    [*r["raw_records"]["records"], {"index": 2, "id": "c9", "sha256": "0" * 64}],
                ),
                ("raw_records", "count"),
                3,
            ),
            expected_receipts=[*_RECORDED, ("c3", "syntax_error")],
            expected_refusals=[
                ("c9", "listed in the run's reconciliation but absent from its records")
            ],
        ),
        ReconciledRunTestCase(
            description="a truncated last raw line",
            edit=lambda r: r,
            expected_receipts=[*_RECORDED, ("c3", "syntax_error")],
            expected_refusals=[
                ("?", "raw record is not JSON"),
                ("c4", "listed in the run's reconciliation but absent from its records"),
            ],
            partial_line=b'{"id": "c4", "sql_sha',
        ),
        ReconciledRunTestCase(
            description="a run with both a complete event and a reconciliation",
            edit=lambda r: r,
            expected_receipts=[],
            expected_refusals=[("*", "audit has a complete event and a reconciliation record")],
            audit_events=3,
        ),
        ReconciledRunTestCase(
            description="a stalled run with neither a complete event nor a reconciliation",
            edit=lambda r: r,
            expected_receipts=[],
            expected_refusals=[("*", "audit has 0 complete events, expected 1")],
            file_name="unbound.json",
        ),
        ReconciledRunTestCase(
            description="a reconciled run that is not a DDL run",
            edit=lambda r: r,
            expected_receipts=[],
            expected_refusals=[("*", "only a DDL run can be reconciled")],
            start={},
        ),
        ReconciledRunTestCase(
            description="a reconciliation that is not JSON",
            edit=lambda r: r,
            expected_receipts=[],
            expected_refusals=[("*", "reconciliation.json is not JSON")],
            render=lambda r: "{",
        ),
        ReconciledRunTestCase(
            description="a reconciliation that is not an object",
            edit=lambda r: r,
            expected_receipts=[],
            expected_refusals=[("*", "reconciliation.json is not a JSON object")],
            render=lambda r: "[]",
        ),
        ReconciledRunTestCase(
            description="another format",
            edit=lambda r: replaced(r, ("format",), "x/1"),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "format")],
        ),
        ReconciledRunTestCase(
            description="another status",
            edit=lambda r: replaced(r, ("status",), "complete"),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "status")],
        ),
        ReconciledRunTestCase(
            description="another run",
            edit=lambda r: replaced(r, ("run_id",), "other"),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "run_id")],
        ),
        ReconciledRunTestCase(
            description="another audit",
            edit=lambda r: replaced(r, ("audit_sha256",), "0" * 64),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "audit_sha256")],
        ),
        ReconciledRunTestCase(
            description="another packet",
            edit=lambda r: replaced(r, ("packet_sha256",), "0" * 64),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "packet_sha256")],
        ),
        ReconciledRunTestCase(
            description="another recording session",
            edit=lambda r: replaced(r, ("recording_session_id",), "S2"),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "recording_session_id")],
        ),
        ReconciledRunTestCase(
            description="no failure",
            edit=lambda r: removed(r, ("failure",)),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "failure")],
        ),
        ReconciledRunTestCase(
            description="no cleanup query id",
            edit=lambda r: removed(r, ("observed_cleanup", "drop_query_id")),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "observed_cleanup.drop_query_id")],
        ),
        ReconciledRunTestCase(
            description="a failed cleanup",
            edit=lambda r: replaced(r, ("observed_cleanup", "drop_status"), "FAILED"),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "observed_cleanup.drop_status")],
        ),
        ReconciledRunTestCase(
            description="a cleanup of another database",
            edit=lambda r: replaced(r, ("observed_cleanup", "dropped"), "SQLQUACK_DDL_OTHER"),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "observed_cleanup.dropped")],
        ),
        ReconciledRunTestCase(
            description="an unequal inventory",
            edit=lambda r: replaced(r, ("inventory_check", "counts_equal"), False),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "inventory_check.counts_equal")],
        ),
        ReconciledRunTestCase(
            description="run-owned names remaining",
            edit=lambda r: replaced(
                r, ("inventory_check", "run_owned_names_remaining"), ["SQLQUACK_DDL_X"]
            ),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "inventory_check.run_owned_names_remaining")],
        ),
        ReconciledRunTestCase(
            description="another raw file",
            edit=lambda r: replaced(r, ("raw_records", "file_sha256"), "0" * 64),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "raw_records.file_sha256")],
        ),
        ReconciledRunTestCase(
            description="raw records that are not a list",
            edit=lambda r: replaced(
                replaced(r, ("raw_records", "records"), "c1"), ("raw_records", "count"), 0
            ),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "raw_records.records")],
        ),
        ReconciledRunTestCase(
            description="a raw record count that differs",
            edit=lambda r: replaced(r, ("raw_records", "count"), 3),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "raw_records.count")],
        ),
        ReconciledRunTestCase(
            description="raw records out of order",
            edit=lambda r: replaced(
                replaced(r, ("raw_records", "records", 0, "index"), 1),
                ("raw_records", "records", 1, "index"),
                0,
            ),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "raw_records.index")],
        ),
        ReconciledRunTestCase(
            description="a raw record listed twice",
            edit=lambda r: replaced(r, ("raw_records", "records", 1, "id"), "c1"),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "raw_records.id")],
        ),
        ReconciledRunTestCase(
            description="a raw record without a hash",
            edit=lambda r: replaced(r, ("raw_records", "records", 0, "sha256"), 5),
            expected_receipts=[],
            expected_refusals=[("*", _HOLD + "raw_records.sha256")],
        ),
        ReconciledRunTestCase(
            description="a submission not from query history",
            edit=lambda r: replaced(r, ("unrecorded_submission", "source"), "connector"),
            expected_receipts=_RECORDED,
            expected_refusals=[("c3", "unrecorded submission is not from query history")],
        ),
        ReconciledRunTestCase(
            description="a submission without a query id",
            edit=lambda r: removed(r, ("unrecorded_submission", "query_id")),
            expected_receipts=_RECORDED,
            expected_refusals=[("c3", "unrecorded submission carries no query-history query id")],
        ),
        ReconciledRunTestCase(
            description="a submission whose SQL hash is not the case SQL",
            edit=lambda r: replaced(
                replaced(r, ("unrecorded_submission", "sql_sha256"), _OTHER_SHA),
                ("unrecorded_submission", "query_text_sha256"),
                _OTHER_SHA,
            ),
            expected_receipts=_RECORDED,
            expected_refusals=[
                ("c3", "unrecorded submission SQL hash does not match the packet's exact SQL")
            ],
        ),
        ReconciledRunTestCase(
            description="a submission whose query text is not the case SQL",
            edit=lambda r: replaced(r, ("unrecorded_submission", "query_text_sha256"), _OTHER_SHA),
            expected_receipts=_RECORDED,
            expected_refusals=[
                ("c3", "unrecorded submission SQL hash does not match the packet's exact SQL")
            ],
        ),
        ReconciledRunTestCase(
            description="a submission carrying a SQLSTATE",
            edit=lambda r: replaced(r, ("unrecorded_submission", "sqlstate"), "42000"),
            expected_receipts=_RECORDED,
            expected_refusals=[
                ("c3", "unrecorded submission carries a SQLSTATE, which query history lacks")
            ],
        ),
        ReconciledRunTestCase(
            description="a submission without a release version",
            edit=lambda r: removed(r, ("unrecorded_submission", "release_version")),
            expected_receipts=_RECORDED,
            expected_refusals=[
                ("c3", "unrecorded submission carries no query-history release_version")
            ],
        ),
        ReconciledRunTestCase(
            description="a submission still running",
            edit=lambda r: replaced(r, ("unrecorded_submission", "execution_status"), "RUNNING"),
            expected_receipts=_RECORDED,
            expected_refusals=[
                (
                    "c3",
                    "unrecorded submission status 'RUNNING' with errno 1003 is not a finished query",
                )
            ],
        ),
        ReconciledRunTestCase(
            description="a successful submission with an error number",
            edit=lambda r: replaced(r, ("unrecorded_submission", "execution_status"), "SUCCESS"),
            expected_receipts=_RECORDED,
            expected_refusals=[
                (
                    "c3",
                    "unrecorded submission status 'SUCCESS' with errno 1003 is not a finished query",
                )
            ],
        ),
        ReconciledRunTestCase(
            description="a failed submission with a text error number",
            edit=lambda r: replaced(r, ("unrecorded_submission", "errno"), "001003"),
            expected_receipts=_RECORDED,
            expected_refusals=[
                (
                    "c3",
                    "unrecorded submission status 'FAILED_WITH_ERROR' with errno '001003' is not a finished query",
                )
            ],
        ),
        ReconciledRunTestCase(
            description="a failed submission without a message",
            edit=lambda r: replaced(r, ("unrecorded_submission", "error_message"), ""),
            expected_receipts=_RECORDED,
            expected_refusals=[
                (
                    "c3",
                    "unrecorded submission status 'FAILED_WITH_ERROR' with errno 1003 is not a finished query",
                )
            ],
        ),
        ReconciledRunTestCase(
            description="a submission without a readable start time",
            edit=lambda r: replaced(r, ("unrecorded_submission", "start_time"), "yesterday"),
            expected_receipts=_RECORDED,
            expected_refusals=[("c3", "unrecorded submission has no readable start time")],
        ),
        ReconciledRunTestCase(
            description="a submission of a case the run recorded",
            edit=lambda r: replaced(
                replaced(
                    replaced(r, ("unrecorded_submission", "case_id"), "c1"),
                    ("unrecorded_submission", "sql_sha256"),
                    _GOOD_SHA,
                ),
                ("unrecorded_submission", "query_text_sha256"),
                _GOOD_SHA,
            ),
            expected_receipts=_RECORDED,
            expected_refusals=[("c1", "unrecorded submission has a raw record")],
        ),
        ReconciledRunTestCase(
            description="a submission that is not an object",
            edit=lambda r: replaced(r, ("unrecorded_submission",), "c3"),
            expected_receipts=_RECORDED,
            expected_refusals=[("?", "unrecorded submission is not an object")],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_run_without_a_complete_event_when_reconciled_then_only_listed_records_import(
    test_case: ReconciledRunTestCase, tmp_path: Path
) -> None:
    run: Path = write_run(
        root=tmp_path,
        rows=_RECONCILED_ROWS,
        packet=_RECONCILED_PACKET,
        start=test_case.start,
        audit_events=test_case.audit_events,
    )
    with (run / "capture" / "raw.jsonl").open("ab") as raw:
        raw.write(test_case.partial_line)
    record: Row = write_reconciliation(
        run=run, unrecorded=unrecorded_submission(case_id="c3", sql=_BROKEN)
    )
    text: str = test_case.render(test_case.edit(record))
    (run / test_case.file_name).write_text(text)
    out: Path = tmp_path / "out"

    run_import(
        [str(tmp_path / "runs"), str(out / "r.jsonl"), str(out / "report.json")]
        + [str(tmp_path / "inputs")]
    )

    receipts: list[Row] = [json.loads(line) for line in (out / "r.jsonl").read_text().splitlines()]
    report: Row = json.loads((out / "report.json").read_text())
    assert [(r["case_id"], r["phase"]) for r in receipts] == test_case.expected_receipts
    assert sorted((r["row_id"], r["reason"]) for r in report["refusals"]) == sorted(
        test_case.expected_refusals
    )
    assert report["reconciled_receipts"] == len(receipts)
    assert (report["runs"][0].get("reconciliation_sha256") == sha(text.encode())) == (
        report["runs"][0]["status"] == "reconciled"
    )


@pytest.mark.parametrize(
    "test_case",
    (
        QueryHistoryReceiptTestCase(
            description="a submission that failed to compile",
            edit=lambda entry: entry,
            expected_receipt={
                "phase": "syntax_error",
                "errno": 1003,
                "sqlstate": None,
                "query_id": "q-unrecorded",
                "warehouse_version": "10.40.100",
                "recorded_at": "2026-11-04T08:18:07.366000+00:00",
            },
        ),
        QueryHistoryReceiptTestCase(
            description="a submission that succeeded",
            edit=lambda entry: {
                **entry,
                "execution_status": "SUCCESS",
                "errno": None,
                "error_message": None,
                "release_version": "10.41.0",
                "start_time": "2026-11-04 09:00:00+01:00",
            },
            expected_receipt={
                "phase": "accepted",
                "errno": None,
                "sqlstate": None,
                "query_id": "q-unrecorded",
                "warehouse_version": "10.41.0",
                "recorded_at": "2026-11-04T09:00:00+01:00",
            },
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_reconciled_query_history_submission_when_imported_then_it_records_only_its_row(
    test_case: QueryHistoryReceiptTestCase, tmp_path: Path
) -> None:
    run: Path = write_run(
        root=tmp_path,
        rows=_RECONCILED_ROWS,
        packet=_RECONCILED_PACKET,
        start={"run_database": "SQLQUACK_DDL_ABC123", "setup": []},
        audit_events=2,
    )
    entry: Row = test_case.edit(unrecorded_submission(case_id="c3", sql=_BROKEN))
    (run / "reconciliation.json").write_text(
        json.dumps(write_reconciliation(run=run, unrecorded=entry))
    )

    imported: Imported = import_runs(runs=tmp_path / "runs", input_dirs=[tmp_path / "inputs"])

    receipt: Row = imported.receipts[-1]
    assert receipt["case_id"] == "c3"
    assert {key: receipt[key] for key in test_case.expected_receipt} == test_case.expected_receipt
    assert receipt["raw_record_sha256"] == sha(canonical_json(entry))
    assert receipt["phase_basis"].endswith(
        "; from query history q-unrecorded, the run stopped before its raw record"
    )
