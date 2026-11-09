"""Recording referee cases against a fake warehouse session."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.referee._helpers.guards import setup_refusal, statement_refusal
from sqlquack_record.referee.main.load_referee import load_referee
from sqlquack_record.referee.models import RefereePending, RefereeSummary
from tests.unit.recorder.src.sqlquack_record.referee._test_types import (
    CatalogueTestCase,
    GuardTestCase,
    MessageTestCase,
    ParityTestCase,
    PendingFileTestCase,
    RefereeTestCase,
)
from tests.unit.recorder.src.sqlquack_record.referee.helpers import (
    catalogue_names,
    outcome,
    outcome_kind,
    record,
    scripted_pending,
    write_pending,
)

_ORDERS: tuple[str, ...] = ("CREATE TEMPORARY TABLE orders (order_id NUMBER(38, 0))",)


@pytest.mark.parametrize(
    "test_case",
    (
        RefereeTestCase(
            description="an accepted statement is explained, then described without running",
            setup_sql=_ORDERS,
            statements=("SELECT order_id FROM orders",),
            syntax_only=False,
            preexisting=(),
            expected_outcomes=("accepted",),
            expected_refused=0,
            expected_log=("open", "create", "explain", "describe", "close"),
        ),
        RefereeTestCase(
            description="a rejected statement records its error and is not described",
            setup_sql=_ORDERS,
            statements=("SELECT order_idd FROM orders", "SELECT order_id FROM orders"),
            syntax_only=False,
            preexisting=(),
            expected_outcomes=("rejected", "accepted"),
            expected_refused=0,
            expected_log=("open", "create", "explain", "explain", "describe", "close"),
        ),
        RefereeTestCase(
            description="a syntax-only statement is never described",
            setup_sql=(),
            statements=("SELECT 1",),
            syntax_only=True,
            preexisting=(),
            expected_outcomes=("accepted",),
            expected_refused=0,
            expected_log=("open", "explain", "close"),
        ),
        RefereeTestCase(
            description="an existing outcome is never re-recorded",
            setup_sql=_ORDERS,
            statements=("SELECT order_id FROM orders", "SELECT 1 FROM orders"),
            syntax_only=False,
            preexisting=("SELECT order_id FROM orders",),
            expected_outcomes=("preexisting", "accepted"),
            expected_refused=0,
            expected_log=("open", "create", "explain", "describe", "close"),
        ),
        RefereeTestCase(
            description="a setup statement other than an empty temporary table refuses its cases",
            setup_sql=("CREATE TABLE orders (order_id NUMBER)",),
            statements=("SELECT order_id FROM orders",),
            syntax_only=False,
            preexisting=(),
            expected_outcomes=("missing",),
            expected_refused=1,
            expected_log=(),
        ),
        RefereeTestCase(
            description="a statement that is not a query is refused without a session",
            setup_sql=_ORDERS,
            statements=("DELETE FROM orders",),
            syntax_only=False,
            preexisting=(),
            expected_outcomes=("missing",),
            expected_refused=1,
            expected_log=(),
        ),
        RefereeTestCase(
            description="an internal compile error refuses only its case and the run goes on",
            setup_sql=_ORDERS,
            statements=("SELECT 'INTERNAL' FROM orders", "SELECT order_id FROM orders"),
            syntax_only=False,
            preexisting=(),
            expected_outcomes=("missing", "accepted"),
            expected_refused=1,
            expected_log=("open", "create", "explain", "explain", "describe", "close"),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_referee_cases_when_recording_then_each_outcome_is_stored_once(
    test_case: RefereeTestCase, tmp_path: Path
) -> None:
    hashes: list[str] = write_pending(
        directory=tmp_path / "pending",
        setup_sql=test_case.setup_sql,
        statements=test_case.statements,
        syntax_only=test_case.syntax_only,
    )
    results: Path = tmp_path / "referee" / "results"
    results.mkdir(parents=True)
    for statement in test_case.preexisting:
        digest: str = hashes[test_case.statements.index(statement)]
        (results / f"{digest}.json").write_text('{"outcome": "preexisting"}', encoding="utf-8")

    summary, log = record(pending=tmp_path / "pending", evidence=tmp_path / "referee")

    outcomes: tuple[str, ...] = tuple(
        outcome_kind(evidence=tmp_path / "referee", digest=digest) for digest in hashes
    )
    assert outcomes == test_case.expected_outcomes, test_case.description
    assert len(summary.refused) == test_case.expected_refused, test_case.description
    assert tuple(log) == test_case.expected_log, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        MessageTestCase(
            description="a binding error keeps its message with session names replaced by kind",
            statement="SELECT order_idd FROM orders",
            syntax_only=False,
            expected_message="SQL compilation error: error line 1 at position 7 invalid "
            "identifier 'ORDER_IDD' in <database>.<schema>",
        ),
        MessageTestCase(
            description="a syntax-only case drops a message that is not a syntax error",
            statement="SELECT order_idd FROM orders",
            syntax_only=True,
            expected_message=None,
        ),
        MessageTestCase(
            description="a syntax-only case keeps a syntax error's message",
            statement="SELECT FROM orders",
            syntax_only=True,
            expected_message="SQL compilation error: syntax error line 1 at position 9 "
            "unexpected 'FROM'.",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_rejected_case_when_recording_then_its_message_names_nothing_private(
    test_case: MessageTestCase, tmp_path: Path
) -> None:
    hashes: list[str] = write_pending(
        directory=tmp_path / "pending",
        setup_sql=_ORDERS,
        statements=(test_case.statement,),
        syntax_only=test_case.syntax_only,
    )

    record(pending=tmp_path / "pending", evidence=tmp_path / "referee")

    error: object = outcome(evidence=tmp_path / "referee", digest=hashes[0])["error"]
    assert isinstance(error, dict), test_case.description
    assert error["message"] == test_case.expected_message, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        CatalogueTestCase(
            description="only built-in functions are stored",
            runs=1,
            expected_names=[["ABS"]],
            expected_last_written=True,
        ),
        CatalogueTestCase(
            description="an unchanged catalogue is stored once",
            runs=2,
            expected_names=[["ABS"]],
            expected_last_written=False,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_catalogue_request_when_recording_then_built_in_functions_are_stored_once(
    test_case: CatalogueTestCase, tmp_path: Path
) -> None:
    write_pending(
        directory=tmp_path / "pending",
        setup_sql=(),
        statements=(),
        syntax_only=True,
        catalogue=True,
    )

    summaries: list[RefereeSummary] = [
        record(pending=tmp_path / "pending", evidence=tmp_path / "referee")[0]
        for _ in range(test_case.runs)
    ]

    assert catalogue_names(tmp_path / "referee") == test_case.expected_names, test_case.description
    assert (summaries[-1].catalogue is not None) == test_case.expected_last_written, (
        test_case.description
    )


@pytest.mark.parametrize(
    "test_case",
    (
        GuardTestCase(
            description="a query after a comment may be compiled",
            statement="-- orders\nWITH o AS (SELECT 1 AS id) SELECT id FROM o;",
            expected_refused=False,
        ),
        GuardTestCase(
            description="a second statement is refused",
            statement="SELECT 1; DROP TABLE orders",
            expected_refused=True,
        ),
        GuardTestCase(
            description="a system function call is refused",
            statement="SELECT SYSTEM$WAIT(1)",
            expected_refused=True,
        ),
        GuardTestCase(
            description="DDL is refused",
            statement="CREATE TABLE orders (id INT)",
            expected_refused=True,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_statement_when_guarding_then_only_single_queries_may_be_compiled(
    test_case: GuardTestCase,
) -> None:
    assert (statement_refusal(test_case.statement) is not None) == test_case.expected_refused, (
        test_case.description
    )


@pytest.mark.parametrize(
    "test_case",
    (
        GuardTestCase(
            description="a column list, including nested type parameters, may run",
            statement='CREATE TEMPORARY TABLE orders (id NUMBER(38, 0) NOT NULL, "Lines" ARRAY(INT))',
            expected_refused=False,
        ),
        GuardTestCase(
            description="a table filled from a query is refused",
            statement="CREATE TEMPORARY TABLE orders (id INT) AS SELECT 1 AS id",
            expected_refused=True,
        ),
        GuardTestCase(
            description="a system function call after the column list is refused",
            statement="CREATE TEMPORARY TABLE orders (id INT) AS SELECT SYSTEM$WAIT(1) AS id",
            expected_refused=True,
        ),
        GuardTestCase(
            description="a clause after the column list is refused",
            statement="CREATE TEMPORARY TABLE orders (id INT) CLUSTER BY (id)",
            expected_refused=True,
        ),
        GuardTestCase(
            description="a column computed from a query is refused",
            statement="CREATE TEMPORARY TABLE orders (id INT AS (SELECT 1))",
            expected_refused=True,
        ),
        GuardTestCase(
            description="a string literal is refused",
            statement="CREATE TEMPORARY TABLE orders (id VARCHAR DEFAULT 'a')",
            expected_refused=True,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_setup_statement_when_guarding_then_only_column_lists_may_run(
    test_case: GuardTestCase,
) -> None:
    assert (setup_refusal(test_case.statement) is not None) == test_case.expected_refused, (
        test_case.description
    )


@pytest.mark.parametrize(
    "test_case",
    (
        PendingFileTestCase(
            description="a case whose statement changed no longer matches its hash",
            folder="cases",
            key="statement",
            value="SELECT 2",
            expected_error="the case hash does not match its content",
        ),
        PendingFileTestCase(
            description="an environment whose setup changed no longer matches its hash",
            folder="environments",
            key="setup_sql",
            value=["CREATE TEMPORARY TABLE customers (customer_id INT)"],
            expected_error="the environment hash does not match its content",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_tampered_pending_file_when_loading_then_it_is_rejected(
    test_case: PendingFileTestCase, tmp_path: Path
) -> None:
    write_pending(
        directory=tmp_path, setup_sql=_ORDERS, statements=("SELECT 1",), syntax_only=False
    )
    path: Path = next((tmp_path / test_case.folder).glob("*.json"))
    document: dict[str, object] = json.loads(path.read_text(encoding="utf-8"))
    document[test_case.key] = test_case.value
    path.write_text(json.dumps(document), encoding="utf-8")

    with pytest.raises(RecorderError) as raised:
        load_referee(tmp_path)

    assert test_case.expected_error in str(raised.value), test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        ParityTestCase(
            description="the repository's pending script and the recorder agree on every hash",
            selection="generated/names.json",
            expected_environments=1,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_pending_written_by_the_script_when_loading_then_every_hash_verifies(
    test_case: ParityTestCase, tmp_path: Path
) -> None:
    pending: RefereePending | None = scripted_pending(
        repository=tmp_path, selection=test_case.selection
    )

    assert pending is not None, test_case.description
    assert len(pending.environments) == test_case.expected_environments, test_case.description
    assert pending.cases, test_case.description
