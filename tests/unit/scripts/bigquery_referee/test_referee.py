"""The BigQuery referee against hand-written BigQuery replies: no network, no credentials."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from scripts.bigquery_referee._helpers import jobs as jobs_module
from scripts.bigquery_referee._helpers import run as run_module
from scripts.bigquery_referee._helpers.jobs import call, create_dataset
from scripts.bigquery_referee._helpers.judge import (
    dry_run_classes,
    error_class,
    execution_mode,
    observe_dry,
    observe_execution,
)
from scripts.bigquery_referee._helpers.names import (
    leaks,
    normalise,
    positions,
    run_names,
    run_project,
    substitute,
)
from scripts.bigquery_referee._helpers.refusals import refusal_rows
from scripts.bigquery_referee._helpers.values import count_of, field, row_of
from scripts.bigquery_referee.constants import (
    INTERNAL_HELD,
    MODE_CASE_DATASETS,
    MODE_DRY_ONLY,
    MODE_EXECUTE,
    MODE_HELD,
    MODE_ROLLED_BACK,
    MODE_TIMED,
    OUTSIDE_HELD,
    PROJECT_VARIABLE,
)
from scripts.bigquery_referee.exceptions import RefereeError, StopRun
from scripts.bigquery_referee.models import (
    EngineMessage,
    ErrorClass,
    Observation,
    RunNames,
    RunPaths,
)
from scripts.bigquery_referee.types import Row
from tests.unit.scripts.bigquery_referee._test_types import (
    CapTestCase,
    CaseDatasetsTestCase,
    DatasetTestCase,
    FailedRunTestCase,
    HeldTestCase,
    LeakTestCase,
    MappingTestCase,
    NormaliseTestCase,
    PolicyTestCase,
    ProjectTestCase,
    RecordTestCase,
    RefusalMessageTestCase,
    RefusalTableTestCase,
    RetryTestCase,
    StopTestCase,
    SubstituteTestCase,
    TargetTestCase,
    TimedTestCase,
)
from tests.unit.scripts.bigquery_referee.helpers import (
    ACCESS_BODY,
    BACKEND_BODY,
    BILLING_BODY,
    CLEAN_BODY,
    DATASET,
    FOR_UPDATE_BODY,
    FUNCTION_ARGUMENT_BODY,
    INTERNAL_BODY,
    NOT_FOUND_BODY,
    PROJECT,
    QUOTA_BODY,
    RATE_BODY,
    RUN_ID,
    SIGNATURE_BODY,
    SYNTAX_BODY,
    UNMAPPED_BODY,
    RecordedBigQuery,
    Sleeps,
    case_row,
    deny,
    internal_on_select_2,
    no_sleep,
    refuse_from_from,
    refuse_unmapped,
    replies_of,
    run_cases,
)

_NAMES: RunNames = run_names(run_id=RUN_ID, project=PROJECT)


@pytest.mark.parametrize(
    "test_case",
    (
        SubstituteTestCase(
            description="a dataset-qualified table",
            sql="SELECT 1 FROM SQLQUACK_DATASET.t1",
            expected_sql=f"SELECT 1 FROM {DATASET}.t1",
        ),
        SubstituteTestCase(
            description="a backquoted path with the project",
            sql="SELECT 1 FROM `SQLQUACK_PROJECT.SQLQUACK_DATASET.t1`",
            expected_sql=f"SELECT 1 FROM `{PROJECT}.{DATASET}.t1`",
        ),
        SubstituteTestCase(
            description="the second dataset is not read as the first plus 2",
            sql="CREATE SCHEMA SQLQUACK_DATASET2",
            expected_sql=f"CREATE SCHEMA {DATASET}_2",
        ),
        SubstituteTestCase(
            description="a token inside a quoted name with a space",
            sql="SELECT 1 FROM `SQLQUACK_DATASET.t1 Q`",
            expected_sql=f"SELECT 1 FROM `{DATASET}.t1 Q`",
        ),
        SubstituteTestCase(
            description="a longer word that starts or ends with a token is not a token",
            sql="SELECT SQLQUACK_DATASETS , xSQLQUACK_PROJECT , SQLQUACK_DATASET_1",
            expected_sql="SELECT SQLQUACK_DATASETS , xSQLQUACK_PROJECT , SQLQUACK_DATASET_1",
        ),
        SubstituteTestCase(
            description="the dashed project token is the project in an unquoted dashed path",
            sql="SELECT 1 FROM sqlquack-project.SQLQUACK_DATASET.t1 , sqlquack-projects",
            expected_sql=f"SELECT 1 FROM {PROJECT}.{DATASET}.t1 , sqlquack-projects",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_case_text_when_substituted_then_whole_tokens_are_replaced(
    test_case: SubstituteTestCase,
) -> None:
    assert substitute(sql=test_case.sql, names=_NAMES) == test_case.expected_sql


@pytest.mark.parametrize(
    "test_case",
    (
        MappingTestCase(
            description="a Syntax error: message the table has no refusal for",
            status=400,
            body=SYNTAX_BODY,
            expected_phase="syntax_error",
            expected_code="SYNTX",
            expected_message=(
                "invalidQuery: Syntax error: Expected end of input but got keyword SELECT at [1:15]"
            ),
        ),
        MappingTestCase(
            description="a parser refusal without the Syntax error: prefix",
            status=400,
            body=FOR_UPDATE_BODY,
            expected_phase="syntax_error",
            expected_code="PR095",
            expected_message="invalidQuery: FOR UPDATE is not supported at [1:10]",
        ),
        MappingTestCase(
            description="a table not found, with the project and dataset normalised",
            status=404,
            body=NOT_FOUND_BODY,
            expected_phase="semantic_error",
            expected_code="NOTFD",
            expected_message=(
                "notFound: Not found: Table SQLQUACK_PROJECT:SQLQUACK_DATASET2.t9 was not found "
                "in location US"
            ),
        ),
        MappingTestCase(
            description="no matching signature",
            status=400,
            body=SIGNATURE_BODY,
            expected_phase="semantic_error",
            expected_code="NOSIG",
            expected_message=(
                "invalidQuery: No matching signature for function ABS for argument types: STRING. "
                "Supported signature: ABS(INT64) at [1:10]"
            ),
        ),
        MappingTestCase(
            description="an invalidQuery message no class matches is unmapped, not guessed",
            status=400,
            body=UNMAPPED_BODY,
            expected_phase="",
            expected_code=None,
            expected_message="invalidQuery: Unexpected ALTER statement in a dry run at [1:1]",
            expected_unmapped=("invalidQuery: Unexpected ALTER statement in a dry run at [1:1]",),
        ),
        MappingTestCase(
            description="a clean dry run of a query",
            status=200,
            body=CLEAN_BODY,
            expected_phase="accepted",
            expected_code=None,
            expected_message=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_recorded_dry_run_reply_when_judged_then_the_phase_follows_its_class(
    test_case: MappingTestCase,
) -> None:
    fake: RecordedBigQuery = RecordedBigQuery(dry=lambda _: (test_case.status, test_case.body))
    observation: Observation = observe_dry(
        case={"case_id": "bq.query.x", "sql": "SELECT 1"}, transport=fake, names=_NAMES
    )
    assert (
        observation.phase,
        observation.engine_code,
        observation.parse_only_code,
        observation.message,
        observation.unmapped,
    ) == (
        test_case.expected_phase,
        test_case.expected_code,
        test_case.expected_code,
        test_case.expected_message,
        test_case.expected_unmapped,
    )


@pytest.mark.parametrize(
    "test_case",
    (
        RefusalMessageTestCase(
            description="FOR UPDATE",
            message="FOR UPDATE is not supported at [1:10]",
            expected_code="PR095",
        ),
        RefusalMessageTestCase(
            description="a query as a function argument",
            message=str(field(value=FUNCTION_ARGUMENT_BODY, path=("error", "message"))),
            expected_code="PR128",
        ),
        RefusalMessageTestCase(
            description="a refusal built from a name",
            message="ALTER SEQUENCE is not supported at [1:7]",
            expected_code="PR012",
        ),
        RefusalMessageTestCase(
            description="EXCEPT with no set modifier, from the parser's own code",
            message='EXCEPT must be followed by ALL, DISTINCT, or "(" [at 1:17]',
            expected_code="PI001",
        ),
        RefusalMessageTestCase(
            description="a reserved keyword as a label, from the parser's own code",
            message="Reserved keyword 'SELECT' may not be used as a label name without backticks"
            " [at 1:1]",
            expected_code="PI002",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_parser_refusal_message_when_classed_then_its_grammar_row_decides(
    test_case: RefusalMessageTestCase,
) -> None:
    found: ErrorClass | None = error_class(
        error=EngineMessage(reason="invalidQuery", message=test_case.message),
        classes=dry_run_classes(),
        said=str,
    )
    assert found is not None
    assert (found.code, found.phase) == (test_case.expected_code, "syntax_error")


@pytest.mark.parametrize(
    "test_case",
    (
        RefusalTableTestCase(
            description="literals, a raw literal, a built message and a bare prefix",
            grammar=(
                'return MakeSyntaxError(@1, "FOR UPDATE is not supported");\n'
                'return MakeSyntaxError(@1, R"(Syntax error: Illegal input character "%s")");\n'
                'return MakeSyntaxError(@kind, absl::StrCat("ALTER ", name, " is not supported"));\n'
                'return MakeSyntaxError(@1, "Syntax error: ");\n'
                'return MakeSyntaxError(@1, "FOR UPDATE is not " "supported");\n'
            ),
            expected_patterns=[
                "^FOR UPDATE is not supported",
                '^Syntax error: Illegal input character ".+?"',
                "^ALTER .+? is not supported",
            ],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_grammar_actions_when_read_then_each_distinct_refusal_is_one_row(
    test_case: RefusalTableTestCase,
) -> None:
    rows: list[Row] = refusal_rows(test_case.grammar)
    assert [row["pattern"] for row in rows] == test_case.expected_patterns


@pytest.mark.parametrize(
    "test_case",
    (
        CapTestCase(
            description="a dry run",
            case_id="bq.query.x",
            sql="SELECT 1",
            expected_dry_runs=[True],
        ),
        CapTestCase(
            description="a dry run then an execution",
            case_id="bq.utility.x",
            sql="BEGIN SELECT 1 ; END",
            expected_dry_runs=[True, False],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_any_job_when_submitted_then_it_caps_bytes_billed(
    test_case: CapTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(run_module, "run_id_now", lambda: RUN_ID)
    fake: RecordedBigQuery = RecordedBigQuery()
    run_module.probe(
        sql=test_case.sql, family=test_case.case_id.split(".")[1], transport=fake, project=PROJECT
    )
    jobs: list[Row] = fake.jobs[-len(test_case.expected_dry_runs) :]
    assert [
        (
            bool(row_of(job["configuration"]).get("dryRun")),
            field(value=job, path=("configuration", "query", "maximumBytesBilled")),
        )
        for job in jobs
    ] == [(dry, "10485760") for dry in test_case.expected_dry_runs]


@pytest.mark.parametrize(
    "test_case",
    (
        StopTestCase("access denied", 403, ACCESS_BODY, "accessDenied"),
        StopTestCase("quota exceeded", 403, QUOTA_BODY, "quotaExceeded"),
        StopTestCase("billing not enabled", 403, BILLING_BODY, "billingNotEnabled"),
    ),
    ids=lambda case: case.description,
)
def test_given_a_run_error_when_dry_running_then_the_run_stops(test_case: StopTestCase) -> None:
    fake: RecordedBigQuery = RecordedBigQuery(dry=lambda _: (test_case.status, test_case.body))
    with pytest.raises(StopRun, match=test_case.expected_match):
        observe_dry(case={"case_id": "bq.query.x", "sql": "SELECT 1"}, transport=fake, names=_NAMES)


@pytest.mark.parametrize(
    "test_case",
    (
        RetryTestCase(
            description="a rate limit and a backend error, then success",
            replies=[(403, RATE_BODY), (503, BACKEND_BODY), (200, CLEAN_BODY)],
            expected_status=200,
            expected_sleeps=2,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_transient_errors_when_called_then_it_retries_with_backoff(
    test_case: RetryTestCase,
) -> None:
    sleeps: Sleeps = Sleeps()
    status, _ = call(
        transport=replies_of(list(test_case.replies)),
        request=("GET", "/x", None),
        names=_NAMES,
        sleep=sleeps,
    )
    assert (status, len(sleeps.waits)) == (test_case.expected_status, test_case.expected_sleeps)


@pytest.mark.parametrize(
    "test_case",
    (
        RetryTestCase(
            description="rate limits past the last attempt",
            replies=[(403, RATE_BODY)] * 6,
            expected_status=403,
            expected_sleeps=5,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_rate_limits_that_persist_when_called_then_the_run_stops(
    test_case: RetryTestCase,
) -> None:
    sleeps: Sleeps = Sleeps()
    with pytest.raises(StopRun, match="rateLimitExceeded"):
        call(
            transport=replies_of(list(test_case.replies)),
            request=("GET", "/x", None),
            names=_NAMES,
            sleep=sleeps,
        )
    assert len(sleeps.waits) == test_case.expected_sleeps


@pytest.mark.parametrize(
    "test_case",
    (
        NormaliseTestCase(
            description="project, both datasets, a job id, an email and a URL",
            sql="SELECT 1",
            message=(
                f"Access to {PROJECT}:{DATASET}_2.t1 and `{PROJECT}.{DATASET}.t2` by "
                "referee@sandbox-project.iam.gserviceaccount.com in job "
                "sqlquack_0123456789abcdef0123456789abcdef failed; see "
                "https://cloud.google.com/bigquery/docs/error-messages"
            ),
            expected_message=(
                "Access to SQLQUACK_PROJECT:SQLQUACK_DATASET2.t1 and "
                "`SQLQUACK_PROJECT.SQLQUACK_DATASET.t2` by <email> in job <job> failed; see <url>"
            ),
        ),
        NormaliseTestCase(
            description="an upper-case dataset name, as some messages spell it",
            sql="SELECT 1",
            message=f"Dataset {DATASET.upper()} was not found",
            expected_message="Dataset SQLQUACK_DATASET was not found",
        ),
        NormaliseTestCase(
            description="a position after a substituted name is the committed text's position",
            sql="SELECT 1 FROM SQLQUACK_DATASET.t1 SELECT",
            message="Syntax error: Expected end of input but got keyword SELECT at [1:42]",
            expected_message="Syntax error: Expected end of input but got keyword SELECT at [1:35]",
        ),
        NormaliseTestCase(
            description="a position inside a substituted name is the token's start",
            sql="SELECT 1 FROM SQLQUACK_DATASET.t1",
            message="Unrecognized name at [1:20]",
            expected_message="Unrecognized name at [1:15]",
        ),
        NormaliseTestCase(
            description="the project by number, as change-history messages name it",
            sql="SELECT 1",
            message=f"Change history is not enabled for table 123456789012:{DATASET}.t1.",
            expected_message="Change history is not enabled for table SQLQUACK_PROJECT:SQLQUACK_DATASET.t1.",
        ),
        NormaliseTestCase(
            description="a time-travel snapshot and the run's earliest readable time",
            sql="SELECT 1",
            message=(
                f"Invalid time travel timestamp 1767225600000 for table {PROJECT}:{DATASET}.t1"
                "@1767225600000. Cannot read before 1794016170384"
            ),
            expected_message=(
                "Invalid time travel timestamp 1767225600000 for table "
                "SQLQUACK_PROJECT:SQLQUACK_DATASET.t1@<snapshot>. Cannot read before <time>"
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_engine_message_when_normalised_then_no_run_name_leaks(
    test_case: NormaliseTestCase,
) -> None:
    found: str = normalise(
        text=test_case.message,
        names=_NAMES,
        position=positions(sql=test_case.sql, names=_NAMES),
    )
    assert (found, leaks(text=found, names=_NAMES)) == (test_case.expected_message, False)


@pytest.mark.parametrize(
    "test_case",
    (
        LeakTestCase("the project", PROJECT, True),
        LeakTestCase("the project by number", "table 123456789012:d.t1", True),
        LeakTestCase("the first dataset", DATASET, True),
        LeakTestCase("the second dataset", f"{DATASET}_2", True),
        LeakTestCase("an email", "a@b.example", True),
        LeakTestCase("a URL", "https://x", True),
        LeakTestCase("a job id", "job_AbCdEfGh123", True),
        LeakTestCase("a plain message", "Syntax error: Unexpected keyword ON at [1:3]", False),
    ),
    ids=lambda case: case.description,
)
def test_given_text_when_checked_then_every_run_name_is_a_leak(test_case: LeakTestCase) -> None:
    assert leaks(text=test_case.text, names=_NAMES) == test_case.expected_leak


@pytest.mark.parametrize(
    "test_case",
    (
        PolicyTestCase("a query", "bq.query.x", "SELECT 1 FROM SQLQUACK_DATASET.t1", MODE_DRY_ONLY),
        PolicyTestCase("a script", "bq.utility.x", "BEGIN SELECT 1 ; END", MODE_EXECUTE),
        PolicyTestCase(
            "an insert",
            "bq.dml.insert.x",
            "INSERT SQLQUACK_DATASET.t1 VALUES ( 1 )",
            MODE_ROLLED_BACK,
        ),
        PolicyTestCase(
            "LOAD DATA", "bq.dml.load-data.x", "LOAD DATA INTO SQLQUACK_DATASET.t1", MODE_HELD
        ),
        PolicyTestCase(
            "TRUNCATE",
            "bq.dml.truncate-table.x",
            "TRUNCATE TABLE SQLQUACK_DATASET.t1",
            MODE_CASE_DATASETS,
        ),
        PolicyTestCase(
            "EXPORT DATA",
            "bq.dml.export-data.x",
            "EXPORT DATA OPTIONS ( uri = 'gs://b/f-*.csv' , format = 'CSV' ) AS SELECT 1",
            MODE_CASE_DATASETS,
        ),
        PolicyTestCase(
            "EXPORT DATA to another bucket",
            "bq.dml.export-data.x",
            "EXPORT DATA OPTIONS ( uri = 'gs://b2/f-*.csv' , format = 'CSV' ) AS SELECT 1",
            MODE_HELD,
        ),
        PolicyTestCase(
            "EXPORT DATA with no uri",
            "bq.dml.export-data.x",
            "EXPORT DATA OPTIONS ( format = 'CSV' ) AS SELECT 1",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a table in another dataset",
            "bq.ddl.create-table.x",
            "CREATE TABLE ds9.t ( c1 INT64 )",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a truncate in another dataset of the project",
            "bq.dml.truncate-table.x",
            "TRUNCATE TABLE `SQLQUACK_PROJECT` . ds9 . t",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a view in another dataset, one quoted path",
            "bq.ddl.create-view.x",
            "CREATE VIEW IF NOT EXISTS `SQLQUACK_PROJECT.ds9.v` AS SELECT 1",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a clone of another dataset's table",
            "bq.ddl.create-table-clone.x",
            "CREATE TABLE SQLQUACK_DATASET.t9 CLONE ds9.t",
            MODE_HELD,
        ),
        PolicyTestCase(
            "an insert into another dataset without INTO",
            "bq.ddl.x",
            "INSERT ds9.t VALUES ( 1 )",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a delete in another dataset without FROM",
            "bq.ddl.x",
            "DELETE ds9.t WHERE TRUE",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a search index on another dataset's table",
            "bq.ddl.create-search-index.x",
            "CREATE SEARCH INDEX i1 ON ds9.t ( ALL COLUMNS )",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a row access policy on another dataset's table",
            "bq.security_admin.create-row-access-policy.x",
            "CREATE ROW ACCESS POLICY p ON `SQLQUACK_PROJECT.ds9.t` FILTER USING ( TRUE )",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a join condition in a view is not a target",
            "bq.ddl.create-view.x",
            "CREATE VIEW SQLQUACK_DATASET.v AS SELECT 1 FROM SQLQUACK_DATASET.t1 a JOIN "
            "SQLQUACK_DATASET.t1 b ON a.c1 = b.c1",
            MODE_CASE_DATASETS,
        ),
        PolicyTestCase(
            "a search index on the run's table",
            "bq.ddl.create-search-index.x",
            "CREATE SEARCH INDEX i1 ON SQLQUACK_DATASET.t1 ( ALL COLUMNS )",
            MODE_CASE_DATASETS,
        ),
        PolicyTestCase(
            "a table in the second dataset, project-qualified",
            "bq.ddl.create-table.x",
            "CREATE TABLE `SQLQUACK_PROJECT` . SQLQUACK_DATASET2 . t9 ( c1 INT64 )",
            MODE_CASE_DATASETS,
        ),
        PolicyTestCase(
            "a model", "bq.ddl.create-model.x", "CREATE MODEL SQLQUACK_DATASET.m1", MODE_HELD
        ),
        PolicyTestCase(
            "any DDL",
            "bq.ddl.create-table.x",
            "CREATE TABLE SQLQUACK_DATASET.t9 ( c1 INT64 )",
            MODE_CASE_DATASETS,
        ),
        PolicyTestCase(
            "dropping the second dataset as external",
            "bq.ddl.drop-schema.x",
            "DROP EXTERNAL SCHEMA IF EXISTS `SQLQUACK_PROJECT` . SQLQUACK_DATASET2",
            MODE_CASE_DATASETS,
        ),
        PolicyTestCase(
            "an external table",
            "bq.ddl.create-external-table.x",
            "CREATE EXTERNAL TABLE SQLQUACK_DATASET.t9 OPTIONS ( uris = [ 'gs://b/f' ] )",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a schema the run does not own",
            "bq.ddl.create-schema.x",
            "CREATE SCHEMA `SELECT`",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a schema named like the second dataset",
            "bq.ddl.create-schema.x",
            "CREATE SCHEMA SQLQUACK_DATASET2_x",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a regional data policy",
            "bq.ddl.create-data-policy.x",
            "CREATE DATA_POLICY `SQLQUACK_PROJECT.region-us.p1` OPTIONS ( )",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a row access policy",
            "bq.security_admin.drop-row-access-policy.x",
            "DROP ALL ROW ACCESS POLICIES ON SQLQUACK_DATASET.t1",
            MODE_CASE_DATASETS,
        ),
        PolicyTestCase(
            "a project's options",
            "bq.security_admin.alter-project-set-options.x",
            "ALTER PROJECT `SQLQUACK_PROJECT` SET OPTIONS ( )",
            MODE_HELD,
        ),
        PolicyTestCase(
            "a reservation",
            "bq.security_admin.x",
            "DROP RESERVATION `SQLQUACK_PROJECT` . 'us' . v1",
            MODE_HELD,
        ),
        PolicyTestCase("a loop", "bq.utility.x", "LOOP SELECT 1 ; END LOOP", MODE_TIMED),
        PolicyTestCase("WHILE TRUE", "bq.utility.x", "WHILE TRUE DO END WHILE", MODE_TIMED),
        PolicyTestCase(
            "DDL in a script",
            "bq.utility.x",
            "BEGIN DROP TABLE SQLQUACK_DATASET.t1 ; END",
            MODE_HELD,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_clean_dry_run_when_the_policy_is_read_then_the_mode_follows(
    test_case: PolicyTestCase,
) -> None:
    assert (
        execution_mode(case_id=test_case.case_id, sql=test_case.sql)[0] == test_case.expected_mode
    )


@pytest.mark.parametrize(
    "test_case",
    (
        TargetTestCase("the run's second dataset", f"{DATASET}_2", ""),
        TargetTestCase("another dataset", "ds9", OUTSIDE_HELD),
    ),
    ids=lambda case: case.description,
)
def test_given_a_ddl_dry_run_when_its_target_is_reported_then_only_the_runs_datasets_run(
    test_case: TargetTestCase,
) -> None:
    target: Row = {"projectId": PROJECT, "datasetId": test_case.dataset, "tableId": "t9"}
    body: Row = {
        **CLEAN_BODY,
        "statistics": {"query": {"statementType": "CREATE_TABLE", "ddlTargetTable": target}},
    }
    fake: RecordedBigQuery = RecordedBigQuery(dry=lambda _: (200, body))
    observation: Observation = observe_dry(
        case={"case_id": "bq.ddl.create-table.x", "sql": "CREATE TABLE x ( c1 INT64 )"},
        transport=fake,
        names=_NAMES,
    )
    assert observation.held == test_case.expected_held


@pytest.mark.parametrize(
    "test_case",
    (
        DatasetTestCase("a sqlload_ dataset", "sqlload_orders"),
        DatasetTestCase("a dataset of another kind", "analytics"),
        DatasetTestCase("a run-shaped name of another run", f"{DATASET}_3"),
    ),
    ids=lambda case: case.description,
)
def test_given_a_dataset_that_is_not_the_runs_when_created_then_it_is_refused_unsent(
    test_case: DatasetTestCase,
) -> None:
    fake: RecordedBigQuery = RecordedBigQuery()
    with pytest.raises(RefereeError, match="refused"):
        create_dataset(transport=fake, dataset=test_case.dataset, names=_NAMES)
    assert len(fake.calls) == test_case.expected_calls


@pytest.mark.parametrize(
    "test_case",
    (
        FailedRunTestCase(
            description="an unmapped error stops the run and is listed",
            dry=refuse_unmapped,
            cases=[
                case_row(case_id="bq.query.a", sql="SELECT 1 FROM SQLQUACK_DATASET.t1"),
                case_row(case_id="bq.ddl.b", sql="ALTER TABLE SQLQUACK_DATASET.t1 SET OPTIONS ( )"),
            ],
            expected_error=RefereeError,
            expected_match=r"no reviewed class:\ninvalidQuery: Unexpected .* x1: bq.ddl.b",
        ),
        FailedRunTestCase(
            description="internalError on every retry for more than five cases stops the run",
            dry=lambda _: (500, INTERNAL_BODY),
            cases=[case_row(case_id=f"bq.query.i{n}", sql=f"SELECT {n}") for n in range(6)],
            expected_error=StopRun,
            expected_match="6 dry runs gave internalError on every retry",
        ),
        FailedRunTestCase(
            description="a permission error stops the run",
            dry=deny,
            cases=[case_row(case_id="bq.query.a", sql="SELECT 1")],
            expected_error=StopRun,
            expected_match="accessDenied",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_failing_run_when_recorded_then_it_writes_no_receipts_and_drops_its_datasets(
    test_case: FailedRunTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(run_module, "referee_commit", lambda _: "0" * 40)
    monkeypatch.setattr(run_module, "run_id_now", lambda: RUN_ID)
    monkeypatch.setattr(jobs_module, "BACKOFF_SECONDS", 0.0)
    fake: RecordedBigQuery = RecordedBigQuery(dry=test_case.dry)
    out: Path = tmp_path / "receipts" / "run.jsonl"
    with pytest.raises(test_case.expected_error, match=test_case.expected_match):
        run_module.record(
            cases=test_case.cases,
            paths=RunPaths(cases=(), out=out, private=tmp_path / "private"),
            transport=fake,
            project=PROJECT,
            workers=2,
        )
    assert (fake.left(), fake.touched(), out.exists()) == (
        set(),
        {_NAMES.dataset, _NAMES.dataset2},
        False,
    )


@pytest.mark.parametrize(
    "test_case",
    (
        RecordTestCase(
            description="a query, a syntax error, a script, an insert and a held LOAD DATA",
            cases=run_cases(),
            expected_phases={"accepted": 3, "syntax_error": 1},
            expected_held=1,
            expected_compared="4 and 4 receipts; 0 differ",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_cases_when_recorded_forward_and_reversed_then_the_receipts_agree(
    test_case: RecordTestCase,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setattr(run_module, "referee_commit", lambda _: "0" * 40)
    monkeypatch.setattr(run_module, "run_id_now", lambda: RUN_ID)
    forward: Path = tmp_path / "forward.jsonl"
    backward: Path = tmp_path / "backward.jsonl"
    report: Row = run_module.record(
        cases=test_case.cases,
        paths=RunPaths(cases=(), out=forward, private=tmp_path / "private"),
        transport=RecordedBigQuery(dry=refuse_from_from),
        project=PROJECT,
        workers=4,
    )
    run_module.record(
        cases=test_case.cases[::-1],
        paths=RunPaths(cases=(), out=backward, private=tmp_path / "private"),
        transport=RecordedBigQuery(dry=refuse_from_from),
        project=PROJECT,
        workers=4,
    )
    differ: list[str] = run_module.compare(first=forward, second=backward)
    published: str = forward.read_text() + json.dumps(report)
    assert (
        report["phases"],
        sum(count_of(held) for held in row_of(report["held"]).values()),
        sorted(row_of(report["bytes"])),
        differ,
        capsys.readouterr().out.strip(),
        leaks(text=published, names=_NAMES),
    ) == (
        test_case.expected_phases,
        test_case.expected_held,
        sorted(
            [
                "dry_run_bytes_billed",
                "dry_run_bytes_processed",
                "executed_bytes_billed",
                "executed_bytes_processed",
            ]
        ),
        [],
        test_case.expected_compared,
        False,
    )


@pytest.mark.parametrize(
    "test_case",
    (
        HeldTestCase(
            description="one case whose dry run gives internalError on every retry",
            cases=[
                case_row(case_id="bq.query.a", sql="SELECT 1"),
                case_row(case_id="bq.query.b", sql="SELECT 2"),
            ],
            expected_phases={"accepted": 1},
            expected_held={INTERNAL_HELD: 1},
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_persistent_internal_error_when_recorded_then_the_case_is_held(
    test_case: HeldTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(run_module, "referee_commit", lambda _: "0" * 40)
    monkeypatch.setattr(run_module, "run_id_now", lambda: RUN_ID)
    monkeypatch.setattr(jobs_module, "BACKOFF_SECONDS", 0.0)
    out: Path = tmp_path / "run.jsonl"
    report: Row = run_module.record(
        cases=test_case.cases,
        paths=RunPaths(cases=(), out=out, private=tmp_path / "private"),
        transport=RecordedBigQuery(dry=internal_on_select_2),
        project=PROJECT,
        workers=2,
    )
    receipts: list[str] = [json.loads(line)["case_id"] for line in out.read_text().splitlines()]
    assert (report["phases"], report["held"], receipts) == (
        test_case.expected_phases,
        test_case.expected_held,
        ["bq.query.a"],
    )


@pytest.mark.parametrize(
    "test_case",
    (
        ProjectTestCase(
            description="unset",
            value="",
            expected_error=f"set {PROJECT_VARIABLE} to the sandbox project id",
        ),
        ProjectTestCase(
            description="not a project id",
            value="Not A Project",
            expected_error=f"set {PROJECT_VARIABLE} to the sandbox project id",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_no_valid_project_variable_when_the_project_is_read_then_the_referee_stops(
    test_case: ProjectTestCase, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv(PROJECT_VARIABLE, test_case.value)
    with pytest.raises(RefereeError, match=test_case.expected_error):
        run_project()


@pytest.mark.parametrize(
    "test_case",
    (
        CaseDatasetsTestCase(
            description="two DDL cases and a TRUNCATE, each in two datasets of its own",
            cases=[
                case_row(case_id="bq.ddl.drop-table.a", sql="DROP TABLE SQLQUACK_DATASET.t1"),
                case_row(case_id="bq.ddl.drop-schema.b", sql="DROP SCHEMA SQLQUACK_DATASET2"),
                case_row(
                    case_id="bq.dml.truncate-table.c", sql="TRUNCATE TABLE SQLQUACK_DATASET.t1"
                ),
            ],
            expected_own={
                f"{DATASET}_c001",
                f"{DATASET}_c001_2",
                f"{DATASET}_c002",
                f"{DATASET}_c002_2",
                f"{DATASET}_c003",
                f"{DATASET}_c003_2",
            },
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_ddl_when_recorded_then_each_case_runs_in_its_own_datasets_and_drops_them(
    test_case: CaseDatasetsTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(run_module, "referee_commit", lambda _: "0" * 40)
    monkeypatch.setattr(run_module, "run_id_now", lambda: RUN_ID)
    fake: RecordedBigQuery = RecordedBigQuery()
    out: Path = tmp_path / "run.jsonl"
    report: Row = run_module.record(
        cases=test_case.cases,
        paths=RunPaths(cases=(), out=out, private=tmp_path / "private"),
        transport=fake,
        project=PROJECT,
        workers=2,
    )
    receipts: list[Row] = [json.loads(line) for line in out.read_text().splitlines()]
    assert (
        set(fake.created) - {_NAMES.dataset, _NAMES.dataset2},
        fake.left(),
        [f"{DATASET}_c00{n}" in sql for n, sql in enumerate(fake.executed, start=1)],
        {r["phase_basis"] for r in receipts},
        report["case_bytes_billed"],
        leaks(text=out.read_text() + json.dumps(report), names=_NAMES),
    ) == (
        test_case.expected_own,
        set(),
        [True] * len(test_case.cases),
        {"dry run accepted the text and it ran in the case's own datasets"},
        {str(case["case_id"]): 0 for case in test_case.cases},
        False,
    )


class _EndlessJob:
    """A job that runs until BigQuery stops it after some polls, or until it is cancelled."""

    def __init__(self, *, polls_before_done: int | None, stop_message: str) -> None:
        self.polls_before_done: int | None = polls_before_done
        self.stop_message: str = stop_message
        self.polls: int = 0
        self.cancels: int = 0
        self.sent: list[Row] = []

    def __call__(self, *, method: str, path: str, body: Row | None) -> tuple[int, Row]:
        if path.split("?")[0].endswith("/cancel"):
            self.cancels += 1
            return 200, {}
        if method == "POST":
            self.sent.append(body or {})
            return 200, {"status": {"state": "RUNNING"}}
        self.polls += 1
        stopped: bool = self.cancels > 0 or self.polls == self.polls_before_done
        if not stopped:
            return 200, {"status": {"state": "RUNNING"}}
        error: Row = {"reason": "stopped", "message": self.stop_message}
        return 200, {"status": {"state": "DONE", "errorResult": error, "errors": [error]}}


@pytest.mark.parametrize(
    "test_case",
    (
        TimedTestCase(
            description="BigQuery's own job timeout",
            polls_before_done=3,
            stop_message="Job execution was cancelled: Job timed out after 10 sec",
            expected_code="TIMEO",
            expected_cancels=0,
        ),
        TimedTestCase(
            description="still running at the cancel wait",
            polls_before_done=None,
            stop_message="Job execution was cancelled: User requested cancellation",
            expected_code="CANCL",
            expected_cancels=1,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_endless_loop_when_executed_then_the_stop_is_its_receipt(
    test_case: TimedTestCase,
) -> None:
    fake: _EndlessJob = _EndlessJob(
        polls_before_done=test_case.polls_before_done, stop_message=test_case.stop_message
    )
    pending: Observation = Observation(
        phase="", basis="", engine_code=None, parse_only_code=None, message=None, mode=MODE_TIMED
    )
    done: Observation = observe_execution(
        case=case_row(case_id="bq.utility.loops.x", sql="LOOP SELECT 1 ; END LOOP"),
        pending=pending,
        transport=fake,
        names=_NAMES,
        sleep=no_sleep,
    )
    assert (
        done.phase,
        done.engine_code,
        fake.cancels,
        field(value=fake.sent[0], path=("configuration", "jobTimeoutMs")),
        field(value=fake.sent[0], path=("configuration", "query", "maximumBytesBilled")),
    ) == ("runtime_error", test_case.expected_code, test_case.expected_cancels, "10000", "10485760")
