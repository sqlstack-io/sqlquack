"""The syntax receipt importer against small, fabricated run directories."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from scripts.syntax_receipts._helpers.hashing import canonical_json
from scripts.syntax_receipts._helpers.receipts import classify, reason_category
from scripts.syntax_receipts.main.import_runs import run_import
from scripts.syntax_receipts.models import Imported
from scripts.syntax_receipts.types import Row
from tests.unit.scripts.syntax_receipts._test_types import (
    BindingTestCase,
    ClassifyTestCase,
    OverwriteTestCase,
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
    sha,
    unrecorded_submission,
    write_reconciliation,
    write_run,
)

_GOOD: str = "BEGIN RETURN 1; END;"
_OTHER: str = "BEGIN RETURN 2; END;"
_BROKEN: str = "CREATE mTASK t AS SELECT 1"
_FIXTURE: list[str] = ['CREATE TABLE "ORDERS" ("ID" INTEGER)']
_SYNTAX_ROW: dict[str, object] = error_row(
    case_id="c1", sql=_GOOD, errno=1003, message="SQL compilation error:\nsyntax error"
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
            ],
            packet=[
                packet_case(case_id="c1", sql=_GOOD),
                packet_case(case_id="c2", sql=_OTHER),
                packet_case(case_id="c3", sql=_OTHER),
            ],
            expected_receipts=[("c1", "syntax_error")],
            expected_refusals=[("c2", "raw"), ("c3", "error"), ("c9", "row")],
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
                "measure": "d73_missing_object_with_setup",
            },
            expected_receipts=[("c1", "accepted", "compile_only_explain")],
            expected_setup_tail=[
                "CREATE SCHEMA <run_database>.<case_schema>",
                "USE SCHEMA <run_database>.<case_schema>",
                *_FIXTURE,
            ],
            expected_refusals=["c2"],
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
    assert setups[receipt_input][-3:] == test_case.expected_setup_tail
    assert [r["row_id"] for r in report["refusals"]] == test_case.expected_refusals
    assert receipts[0]["measure"] == test_case.start["measure"]


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


def _unlist_second(record: Row) -> None:
    raw: Row = record["raw_records"]
    raw["records"] = raw["records"][:1]
    raw["count"] = 1


def _change_first_hash(record: Row) -> None:
    record["raw_records"]["records"][0]["sha256"] = "0" * 64


def _list_absent(record: Row) -> None:
    raw: Row = record["raw_records"]
    raw["records"].append({"index": 2, "id": "c9", "sha256": "0" * 64})
    raw["count"] = 3


@pytest.mark.parametrize(
    "test_case",
    (
        ReconciledRunTestCase(
            description="a stalled run's listed records and its query-history submission",
            edit=lambda record: None,
            expected_receipts=["c1", "c2", "c3"],
            expected_refusals=[],
        ),
        ReconciledRunTestCase(
            description="a raw record the reconciliation does not list",
            edit=_unlist_second,
            expected_receipts=["c1", "c3"],
            expected_refusals=[("c2", "raw record is not listed in the run's reconciliation")],
        ),
        ReconciledRunTestCase(
            description="a raw record whose hash differs from the reconciliation",
            edit=_change_first_hash,
            expected_receipts=["c2", "c3"],
            expected_refusals=[("c1", "raw record hash differs from the run's reconciliation")],
        ),
        ReconciledRunTestCase(
            description="a listed record the run never recorded",
            edit=_list_absent,
            expected_receipts=["c1", "c2", "c3"],
            expected_refusals=[("c9", "listed in the run's reconciliation but absent")],
        ),
        ReconciledRunTestCase(
            description="a query-history submission without a query id",
            edit=lambda record: record["unrecorded_submission"].pop("query_id"),
            expected_receipts=["c1", "c2"],
            expected_refusals=[("c3", "carries no query-history query id")],
        ),
        ReconciledRunTestCase(
            description="a query-history submission whose SQL hash is not the case SQL",
            edit=lambda record: record["unrecorded_submission"].update(
                sql_sha256=sha(_OTHER.encode()), query_text_sha256=sha(_OTHER.encode())
            ),
            expected_receipts=["c1", "c2"],
            expected_refusals=[("c3", "SQL hash does not match the packet's exact SQL")],
        ),
        ReconciledRunTestCase(
            description="a query-history submission whose query text is not the case SQL",
            edit=lambda record: record["unrecorded_submission"].update(
                query_text_sha256=sha(_OTHER.encode())
            ),
            expected_receipts=["c1", "c2"],
            expected_refusals=[("c3", "unrecorded submission SQL hash does not match")],
        ),
        ReconciledRunTestCase(
            description="a run with both a complete event and a reconciliation",
            edit=lambda record: None,
            complete=True,
            expected_receipts=[],
            expected_refusals=[("*", "a complete event and a reconciliation record")],
        ),
        ReconciledRunTestCase(
            description="a reconciliation bound to another audit",
            edit=lambda record: record.update(audit_sha256="0" * 64),
            expected_receipts=[],
            expected_refusals=[("*", "does not bind this run: audit_sha256")],
        ),
        ReconciledRunTestCase(
            description="a reconciliation without cleanup evidence",
            edit=lambda record: record["observed_cleanup"].pop("drop_query_id"),
            expected_receipts=[],
            expected_refusals=[("*", "has no observed_cleanup.drop_query_id")],
        ),
        ReconciledRunTestCase(
            description="a reconciliation without an inventory check",
            edit=lambda record: record.pop("inventory_check"),
            expected_receipts=[],
            expected_refusals=[("*", "has no inventory_check")],
        ),
        ReconciledRunTestCase(
            description="a stalled run with neither a complete event nor a reconciliation",
            edit=lambda record: None,
            reconcile=False,
            expected_receipts=[],
            expected_refusals=[("*", "0 complete events")],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_run_without_a_complete_event_when_reconciled_then_only_listed_records_import(
    test_case: ReconciledRunTestCase, tmp_path: Path
) -> None:
    run: Path = write_run(
        root=tmp_path,
        rows=[
            {**accepted_row(case_id="c1", sql=_GOOD), "case_schema": "C0000"},
            {**error_row(case_id="c2", sql=_OTHER, errno=2003, message="SQL compilation error: x")},
        ],
        packet=[
            packet_case(case_id="c1", sql=_GOOD),
            packet_case(case_id="c2", sql=_OTHER),
            packet_case(case_id="c3", sql=_BROKEN),
        ],
        start={"run_database": "SQLQUACK_DDL_ABC123", "setup": []},
        complete=test_case.complete,
    )
    record: Row = write_reconciliation(
        run=run, unrecorded=unrecorded_submission(case_id="c3", sql=_BROKEN)
    )
    test_case.edit(record)
    if test_case.reconcile:
        (run / "reconciliation.json").write_text(json.dumps(record))
    out: Path = tmp_path / "out"

    run_import(
        [
            str(tmp_path / "runs"),
            str(out / "r.jsonl"),
            str(out / "report.json"),
            str(tmp_path / "inputs"),
        ]
    )

    receipts: list[Row] = [json.loads(line) for line in (out / "r.jsonl").read_text().splitlines()]
    report: Row = json.loads((out / "report.json").read_text())
    assert [r["case_id"] for r in receipts] == test_case.expected_receipts
    assert len(report["refusals"]) == len(test_case.expected_refusals), report["refusals"]
    for refused, (row_id, fragment) in zip(
        report["refusals"], test_case.expected_refusals, strict=True
    ):
        assert (refused["row_id"], fragment in refused["reason"]) == (row_id, True), refused
    reconciled: bool = report["runs"][0]["status"] == "reconciled"
    assert report["reconciled_receipts"] == (len(receipts) if reconciled else 0)
    for row in receipts:
        if row["case_id"] == "c3":
            assert (row["phase"], row["sqlstate"], row["query_id"], row["recorded_at"]) == (
                "syntax_error",
                None,
                "q-unrecorded",
                "2026-11-04T08:18:07.366000+00:00",
            )
            assert row["raw_record_sha256"] == sha(canonical_json(record["unrecorded_submission"]))
