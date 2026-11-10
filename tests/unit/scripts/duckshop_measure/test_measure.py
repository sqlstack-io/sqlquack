"""DuckShop's census relations, and unit-test outcomes from real DuckDB runs of converted SQL."""

from __future__ import annotations

import datetime
import json
from pathlib import Path

import duckdb
import pyarrow
import pyarrow.ipc
import pytest

from scripts.duckshop_measure._helpers.measures import unit_test_outcomes
from scripts.duckshop_measure._helpers.outcomes import unit_test_outcome
from scripts.duckshop_measure._helpers.relations import census_relations
from scripts.duckshop_measure._helpers.report import unit_tests_report
from scripts.duckshop_measure._helpers.running import duckdb_rows
from scripts.duckshop_measure._helpers.snowflake import (
    load_inputs,
    model_result,
    orders_rows,
    with_unloadable_input,
)
from scripts.duckshop_measure.main.measure import run_measure
from scripts.duckshop_measure.models import (
    Conversion,
    FunctionMacro,
    ModelResult,
    ProjectFunction,
    UnitTestOutcome,
)
from tests.unit.scripts.duckshop_measure._test_types import (
    FunctionMeasureTestCase,
    FunctionSchemaTestCase,
    LoadTestCase,
    ModelTestCase,
    OrderTestCase,
    OutcomeTestCase,
    RunTestCase,
    SchemaTestCase,
    SnowflakeTestCase,
)

_STEPS: str = "SELECT {0} AS step_index, 'm' AS model_name, {1}, {2}, {3}, {4}"
_REFUSAL: str = "missing rule: fn:SOUNDEX"
_ROWS: dict[str, object] = {"model": "m", "target": "core", "outcome": "rows"}


@pytest.mark.parametrize(
    "test_case",
    (
        SchemaTestCase(
            description="parameterised types stay whole and names fold to upper case",
            setup_sql=(
                "CREATE TEMPORARY TABLE rel__shop__orders (order_id VARCHAR NOT NULL, "
                '"Total" NUMBER(38,2), placed_at TIMESTAMP_TZ)',
            ),
            expected_relations={
                "__SQLQUACK_RELATION__SHOP__ORDERS": [
                    ["ORDER_ID", "VARCHAR"],
                    ["TOTAL", "NUMBER(38,2)"],
                    ["PLACED_AT", "TIMESTAMP_TZ"],
                ]
            },
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_environment_when_deriving_the_census_relations_then_each_table_is_one(
    test_case: SchemaTestCase,
) -> None:
    relations: dict[str, list[list[str]]] = census_relations(list(test_case.setup_sql))
    assert relations == test_case.expected_relations


@pytest.mark.parametrize(
    "test_case",
    (
        OutcomeTestCase(
            description="every step with no unexpected or missing rows and equal counts matches",
            converted=True,
            python_functions=False,
            sql=f"{_STEPS.format(0, 3, 3, 0, 0)} UNION ALL {_STEPS.format(1, 0, 0, 0, 0)}",
            expected_outcome="matches",
            expected_details=(),
        ),
        OutcomeTestCase(
            description="a missing row in one step differs, naming that step",
            converted=True,
            python_functions=False,
            sql=f"{_STEPS.format(0, 3, 3, 0, 0)} UNION ALL {_STEPS.format(1, 2, 3, 0, 1)}",
            expected_outcome="differs",
            expected_details=("1, m, 2, 3, 0, 1",),
        ),
        OutcomeTestCase(
            description="unequal counts differ even with no unexpected or missing distinct rows",
            converted=True,
            python_functions=False,
            sql=_STEPS.format(0, 4, 3, 0, 0),
            expected_outcome="differs",
            expected_details=("0, m, 4, 3, 0, 0",),
        ),
        OutcomeTestCase(
            description="a divergence guard raising is named, not counted as an error",
            converted=True,
            python_functions=False,
            sql="SELECT CAST('json_fidelity#7f0135058d2 may differ' AS INTEGER)",
            expected_outcome="guard_raised",
            expected_details=("json_fidelity",),
        ),
        OutcomeTestCase(
            description="SQL DuckDB cannot run is a DuckDB error",
            converted=True,
            python_functions=False,
            sql="SELECT CAST('2999-12-31 00:00:00' AS BIGINT)",
            expected_outcome="duckdb_error",
            expected_details=None,
        ),
        OutcomeTestCase(
            description="a refused conversion keeps its constructs and is never run",
            converted=False,
            python_functions=False,
            sql="SELECT not run",
            expected_outcome="refused",
            expected_details=(_REFUSAL,),
        ),
        OutcomeTestCase(
            description="a conversion calling Python functions is not run",
            converted=True,
            python_functions=True,
            sql="SELECT not run",
            expected_outcome="needs_python",
            expected_details=(),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_conversion_when_running_it_on_duckdb_then_its_outcome_is_named(
    test_case: OutcomeTestCase,
) -> None:
    conversion: Conversion = Conversion(
        converted=test_case.converted,
        sql=test_case.sql,
        refusals=(_REFUSAL,),
        python_functions=test_case.python_functions,
    )
    outcome: UnitTestOutcome = unit_test_outcome(
        test="t.sql", conversion=conversion, execute=duckdb_rows
    )
    assert outcome.outcome == test_case.expected_outcome
    assert test_case.expected_details in (None, outcome.details)


@pytest.mark.parametrize(
    "test_case",
    (
        RunTestCase(
            description="each compiled test is reported once, by its path, in path order",
            files=(("b/second.sql", False), ("a/first.sql", True)),
            expected_outcomes={
                "matches": 1,
                "differs": 0,
                "guard_raised": 0,
                "duckdb_error": 0,
                "refused": 1,
                "needs_python": 0,
            },
            expected_order=("a/first.sql", "b/second.sql"),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_compiled_tests_when_measuring_then_every_test_is_counted_once(
    test_case: RunTestCase, tmp_path: Path
) -> None:
    converted: dict[str, bool] = dict(test_case.files)
    for name in converted:
        path: Path = tmp_path / "tests" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(_STEPS.format(0, 1, 1, 0, 0))

    def convert(path: Path) -> Conversion:
        return Conversion(
            converted=converted[str(path.relative_to(tmp_path / "tests"))],
            sql=path.read_text(),
            refusals=(_REFUSAL,),
            python_functions=False,
        )

    outcomes: list[UnitTestOutcome] = unit_test_outcomes(
        compiled=tmp_path, convert=convert, execute=duckdb_rows
    )
    assert unit_tests_report(outcomes)["outcomes"] == test_case.expected_outcomes
    assert tuple(outcome.test for outcome in outcomes) == test_case.expected_order


@pytest.mark.parametrize(
    "test_case",
    (
        OrderTestCase(
            description="a top-level ORDER BY orders the rows",
            sql="SELECT a FROM t ORDER BY a",
            expected_ordered=True,
        ),
        OrderTestCase(
            description="an ORDER BY inside a window or subquery does not",
            sql="SELECT ROW_NUMBER() OVER (ORDER BY a) FROM (SELECT a FROM t ORDER BY a)",
            expected_ordered=False,
        ),
        OrderTestCase(
            description="ORDER BY in a comment or quoted text does not",
            sql="SELECT 'order by' AS \"ORDER BY\" FROM t -- ORDER BY a\n/* ORDER BY b */",
            expected_ordered=False,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_compiled_sql_when_reading_its_order_then_only_a_top_level_order_by_counts(
    test_case: OrderTestCase,
) -> None:
    assert orders_rows(test_case.sql) is test_case.expected_ordered


@pytest.mark.parametrize(
    "test_case",
    (
        ModelTestCase(
            description="a refused model keeps its constructs",
            sidecar=_ROWS,
            refusals=(_REFUSAL,),
            compared=None,
            error=None,
            expected_outcome="refused",
        ),
        ModelTestCase(
            description="a Snowflake error stays apart even when the conversion is refused",
            sidecar={**_ROWS, "outcome": "error"},
            refusals=(_REFUSAL,),
            compared=None,
            error=None,
            expected_outcome="snowflake_error",
        ),
        ModelTestCase(
            description="a conversion relying on a value divergence is never compared",
            sidecar=_ROWS,
            refusals=None,
            compared=None,
            error=None,
            expected_outcome="declared_divergence",
            declared=("hash",),
        ),
        ModelTestCase(
            description="an incremental read from its table is not scored",
            sidecar={**_ROWS, "executed_as": "materialized_relation"},
            refusals=None,
            compared=None,
            error=None,
            expected_outcome="incremental",
        ),
        ModelTestCase(
            description="a nondeterministic model is reported, not scored, even when it matches",
            sidecar={**_ROWS, "nondeterministic": True},
            refusals=None,
            compared={"outcome": "matches", "detail": ""},
            error=None,
            expected_outcome="nondeterministic",
        ),
        ModelTestCase(
            description="a missing recorded input is not a DuckDB error of the conversion",
            sidecar=_ROWS,
            refusals=None,
            compared={
                "outcome": "duckdb_error",
                "detail": "Catalog Error: Table with name __sqlquack_relation__x does not exist!",
            },
            error=None,
            expected_outcome="input_unrecorded",
        ),
        ModelTestCase(
            description="an input DuckDB could not hold at its declared type is input_unloadable",
            sidecar=_ROWS,
            refusals=None,
            compared={
                "outcome": "duckdb_error",
                "detail": "Catalog Error: Table with name __sqlquack_relation__x does not exist!",
            },
            error=None,
            expected_outcome="input_unloadable",
            unloadable={"__sqlquack_relation__x": "ConversionException: out of range"},
        ),
        ModelTestCase(
            description="another missing input stays input_unrecorded beside an unloadable one",
            sidecar=_ROWS,
            refusals=None,
            compared={
                "outcome": "duckdb_error",
                "detail": "Catalog Error: Table with name __sqlquack_relation__y does not exist!",
            },
            error=None,
            expected_outcome="input_unrecorded",
            unloadable={"__sqlquack_relation__x": "ConversionException: out of range"},
        ),
        ModelTestCase(
            description="a project function that could not be created is input_unloadable",
            sidecar=_ROWS,
            refusals=None,
            compared={
                "outcome": "duckdb_error",
                "detail": "Catalog Error: Scalar Function with name __sqlquack_udf__f does not "
                "exist!",
            },
            error=None,
            expected_outcome="input_unloadable",
            unloadable={"__sqlquack_udf__f": "sqlquack function refused: missing rule"},
        ),
        ModelTestCase(
            description="a microsecond-only match is its own outcome",
            sidecar=_ROWS,
            refusals=None,
            compared={"outcome": "matches_at_microseconds", "detail": ""},
            error=None,
            expected_outcome="matches_at_microseconds",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_recorded_model_when_judging_it_then_only_comparable_runs_are_scored(
    test_case: ModelTestCase,
) -> None:
    result: ModelResult = model_result(
        sidecar=test_case.sidecar,
        refusals=test_case.refusals,
        compared=with_unloadable_input(
            compared=test_case.compared, unloadable=test_case.unloadable
        ),
        error=test_case.error,
        declared=test_case.declared,
    )
    assert result.outcome == test_case.expected_outcome


@pytest.mark.parametrize(
    "test_case",
    (
        LoadTestCase(
            description="nanosecond timestamps inside DuckDB's range load",
            rows=(datetime.datetime(2026, 11, 4, 12, 0),),
            expected_unloadable=(),
            expected_loaded=("__sqlquack_relation__edge", "__sqlquack_relation__plain"),
        ),
        LoadTestCase(
            description="year 0001 as TIMESTAMP_NS is unloadable and its neighbour still loads",
            rows=(datetime.datetime(2026, 11, 4, 12, 0), datetime.datetime(1, 1, 1)),
            expected_unloadable=("__sqlquack_relation__edge",),
            expected_loaded=("__sqlquack_relation__plain",),
        ),
        LoadTestCase(
            description="year 9999 as TIMESTAMP_NS is unloadable",
            rows=(datetime.datetime(9999, 12, 31, 23, 59, 59),),
            expected_unloadable=("__sqlquack_relation__edge",),
            expected_loaded=("__sqlquack_relation__plain",),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_recorded_inputs_when_loading_then_only_values_duckdb_cannot_hold_are_refused(
    test_case: LoadTestCase, tmp_path: Path
) -> None:
    entries: list[dict[str, object]] = []
    for name, values, scale in (
        ("edge", test_case.rows, 9),
        ("plain", (datetime.datetime(2026, 1, 1),), 6),
    ):
        table: pyarrow.Table = pyarrow.table(
            {"SENT_AT": pyarrow.array(values, pyarrow.timestamp("us"))}
        )
        with pyarrow.ipc.new_file(tmp_path / f"k_{name}.arrow", table.schema) as writer:
            writer.write_table(table)
        column: dict[str, object] = {
            "name": "SENT_AT",
            "type_name": "TIMESTAMP_NTZ",
            "scale": scale,
        }
        (tmp_path / f"k_{name}.json").write_text(json.dumps({"columns": [column]}))
        entries.append({"key": f"k_{name}", "unique_id": f"model.shop.{name}", "outcome": "rows"})
    database: Path = tmp_path / "inputs.duckdb"
    unloadable: dict[str, str] = load_inputs(entries=entries, results=tmp_path, database=database)
    connection: duckdb.DuckDBPyConnection = duckdb.connect(str(database), read_only=True)
    loaded: tuple[str, ...] = tuple(
        sorted(
            row[0]
            for row in connection.execute("SELECT table_name FROM duckdb_tables()").fetchall()
        )
    )
    connection.close()
    assert (tuple(sorted(unloadable)), loaded) == (
        test_case.expected_unloadable,
        test_case.expected_loaded,
    )
    assert all(reason.startswith("ConversionException") for reason in unloadable.values())


@pytest.mark.parametrize(
    "test_case",
    (
        SnowflakeTestCase(
            description="a loud divergence is compared and a value divergence is declared",
            policy='[divergences.session_timezone_utc]\nfailure = "value"\n\n'
            '[divergences.numeric_range]\nfailure = "loud"\n',
            expected_compared=("k_m_loud",),
            expected_scored=2,
            expected_outcomes={
                "m_loud": ["matches"],
                "m_refused": ["refused", _REFUSAL],
                "m_value": ["declared_divergence", "session_timezone_utc"],
            },
        ),
        SnowflakeTestCase(
            description="a divergence the policy marks value is never compared",
            policy='[divergences.session_timezone_utc]\nfailure = "value"\n\n'
            '[divergences.numeric_range]\nfailure = "value"\n',
            expected_compared=(),
            expected_scored=1,
            expected_outcomes={
                "m_loud": ["declared_divergence", "numeric_range"],
                "m_refused": ["refused", _REFUSAL],
                "m_value": ["declared_divergence", "session_timezone_utc"],
            },
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_recorded_models_when_measuring_snowflake_then_the_policy_decides_what_is_compared(
    test_case: SnowflakeTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    results: Path = tmp_path / "results"
    compiled: Path = tmp_path / "project" / "target" / "compiled"
    converted: Path = tmp_path / "converted"
    guards: Path = tmp_path / "guards"
    for directory in (results, compiled / "models", converted, guards):
        directory.mkdir(parents=True)
    policy: Path = tmp_path / "sqlquack.toml"
    policy.write_text(test_case.policy)
    entries: list[dict[str, object]] = []
    for model in ("m_refused", "m_value", "m_loud"):
        (compiled / "models" / f"{model}.sql").write_text("SELECT 1 AS a ORDER BY a")
        sidecar: dict[str, object] = {
            "key": f"k_{model}",
            "model": model,
            "target": "dev",
            "outcome": "rows",
            "compiled_sql_path": f"benchmarks/x/target/compiled/models/{model}.sql",
        }
        (results / f"k_{model}.json").write_text(json.dumps(sidecar))
        entries.append({"key": f"k_{model}", "unique_id": f"model.shop.{model}", "outcome": "rows"})
    for model, divergence in (("m_value", "session_timezone_utc"), ("m_loud", "numeric_range")):
        name: str = str(compiled / "models" / f"{model}.sql").replace("/", "_")
        (converted / name).write_text("SELECT 1 AS a")
        (guards / f"{name}.json").write_text(json.dumps({"divergences": {divergence: []}}))
    (tmp_path / "index-core.json").write_text(json.dumps(entries))
    (tmp_path / "index-edge.json").write_text("[]")
    census: Path = tmp_path / "census.json"
    census.write_text(
        json.dumps(
            {
                "blockers": {
                    "files": [
                        {
                            "model": "m_refused",
                            "own": [{"category": "missing rule", "construct": "fn:SOUNDEX"}],
                        }
                    ]
                }
            }
        )
    )
    compared_ids: list[object] = []

    def compare(
        *, manifest: Path, comparisons: list[dict[str, object]]
    ) -> dict[str, dict[str, str]]:
        compared_ids.extend(comparison["id"] for comparison in comparisons)
        return {"k_m_loud": {"id": "k_m_loud", "outcome": "matches", "detail": ""}}

    out: Path = tmp_path / "snowflake.json"
    status: int = run_measure(
        argv=[
            "snowflake",
            "--results",
            str(results),
            "--compiled",
            str(compiled),
            "--census",
            str(census),
            "--converted",
            str(converted),
            "--guards",
            str(guards),
            "--sqlquack",
            "unused",
            "--library",
            "unused",
            "--evidence",
            str(policy),
            "--work",
            str(tmp_path / "work"),
            "--out",
            str(out),
        ],
        comparer=compare,
    )
    report: dict[str, object] = json.loads(out.read_text())
    listed: object = report["results"]
    assert isinstance(listed, list)
    outcomes: dict[str, list[str]] = {
        str(item["model"]): [item["outcome"], *item["details"]] for item in listed
    }
    assert (status, tuple(compared_ids), report["scored"]) == (
        0,
        test_case.expected_compared,
        test_case.expected_scored,
    )
    assert outcomes == test_case.expected_outcomes
    assert "3 models" in capsys.readouterr().out


@pytest.mark.parametrize(
    "test_case",
    (
        FunctionSchemaTestCase(
            description="a SQL function is declared under its census placeholder",
            header="CREATE OR REPLACE FUNCTION SHOP.PUBLIC.SKU_FAMILY(sku STRING)\nRETURNS STRING\n"
            "AS $$ split_part(sku, '-', 1) $$",
            expected_functions={
                "functions/sql/__SQLQUACK_UDF__SKU_FAMILY.sql": (
                    "FUNCTION (\n  arguments (SKU STRING),\n  returns STRING,\n);\nNULL\n"
                )
            },
        ),
        FunctionSchemaTestCase(
            description="a Python function is declared with @udf",
            header="CREATE FUNCTION S.P.SKU_FAMILY(x NUMBER) RETURNS NUMBER LANGUAGE PYTHON "
            "HANDLER = 'f' AS $$\ndef f(x):\n    return x\n$$",
            expected_functions={
                "functions/python/__SQLQUACK_UDF__SKU_FAMILY.py": (
                    "from sqlquack import udf\n\n\n"
                    '@udf(arguments={"X": "NUMBER"}, returns="NUMBER")\n'
                    "def main(*arguments):\n    return None\n"
                )
            },
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_compiled_functions_when_writing_the_project_then_they_sit_beside_the_sources(
    test_case: FunctionSchemaTestCase, tmp_path: Path
) -> None:
    environment: Path = tmp_path / "environment.json"
    environment.write_text(
        json.dumps({"setup_sql": ["CREATE TEMPORARY TABLE rel__shop__skus (sku VARCHAR)"]})
    )
    functions: Path = tmp_path / "functions"
    functions.mkdir()
    (functions / "sku_family.sql").write_text(test_case.header)
    out: Path = tmp_path / "project"
    status: int = run_measure(
        argv=[
            "project",
            "--environment",
            str(environment),
            "--functions",
            str(functions),
            "--out",
            str(out),
        ]
    )
    written: dict[str, str] = {
        str(path.relative_to(out)): path.read_text()
        for path in filter(Path.is_file, out.rglob("*"))
    }
    assert (status, written) == (
        0,
        {
            "sources/census.yml": json.dumps(
                {
                    "sources": [
                        {
                            "name": '"__SQLQUACK_RELATION__SHOP__SKUS"',
                            "columns": [{"name": '"SKU"', "type": "VARCHAR"}],
                        }
                    ]
                },
                indent=1,
            )
            + "\n",
            **test_case.expected_functions,
        },
    )


@pytest.mark.parametrize(
    "test_case",
    (
        FunctionMeasureTestCase(
            description="a SQL function runs as a macro and a Python one leaves its model unmeasured",
            models=(
                ("m_sql", "SELECT \"__SQLQUACK_UDF__SKU_FAMILY\"('ab-1') AS a"),
                ("m_py", 'SELECT "__SQLQUACK_UDF__PY_SCORE"(1) AS a'),
            ),
            expected_converted=["__SQLQUACK_UDF__SKU_FAMILY"],
            expected_compared={"k_m_sql": [("AB-1",)]},
            expected_outcomes={
                "m_sql": ["matches"],
                "m_py": ["python_udf_not_measured", "__SQLQUACK_UDF__PY_SCORE"],
            },
        ),
        FunctionMeasureTestCase(
            description="a model census refuses for its unnamed fn:<udf> is still a Python caller",
            models=(),
            refused=(("m_refused", 'SELECT __udf("py_score")(1) AS a'),),
            expected_converted=["__SQLQUACK_UDF__SKU_FAMILY"],
            expected_compared={},
            expected_outcomes={
                "m_refused": ["python_udf_not_measured", "__SQLQUACK_UDF__PY_SCORE"],
            },
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_project_functions_when_measuring_snowflake_then_sql_ones_run_and_python_ones_are_not_measured(
    test_case: FunctionMeasureTestCase, tmp_path: Path
) -> None:
    results: Path = tmp_path / "results"
    compiled: Path = tmp_path / "project" / "target" / "compiled"
    converted: Path = tmp_path / "converted"
    for directory in (results, compiled / "models", compiled / "functions", converted):
        directory.mkdir(parents=True)
    (compiled / "functions" / "sku_family.sql").write_text(
        "CREATE FUNCTION S.P.SKU_FAMILY(sku STRING) RETURNS STRING AS $$ upper(sku) $$"
    )
    (compiled / "functions" / "py_score.sql").write_text(
        "CREATE FUNCTION S.P.PY_SCORE(x NUMBER) RETURNS NUMBER LANGUAGE PYTHON "
        "RUNTIME_VERSION = '3.11' HANDLER = 'f' AS $$\ndef f(x):\n    return x\n$$"
    )
    entries: list[dict[str, object]] = []
    for model, sql in test_case.models:
        (compiled / "models" / f"{model}.sql").write_text("SELECT 1 AS a")
        sidecar: dict[str, object] = {
            "key": f"k_{model}",
            "model": model,
            "target": "dev",
            "outcome": "rows",
            "compiled_sql_path": f"benchmarks/x/target/compiled/models/{model}.sql",
        }
        (results / f"k_{model}.json").write_text(json.dumps(sidecar))
        entries.append({"key": f"k_{model}", "unique_id": f"model.shop.{model}", "outcome": "rows"})
        (converted / str(compiled / "models" / f"{model}.sql").replace("/", "_")).write_text(sql)
    own: list[dict[str, object]] = []
    for model, source in test_case.refused:
        (compiled / "models" / f"{model}.sql").write_text(source)
        refused_sidecar: dict[str, object] = {
            "key": f"k_{model}",
            "model": model,
            "target": "dev",
            "outcome": "rows",
            "compiled_sql_path": f"benchmarks/x/target/compiled/models/{model}.sql",
        }
        (results / f"k_{model}.json").write_text(json.dumps(refused_sidecar))
        entries.append({"key": f"k_{model}", "unique_id": f"model.shop.{model}", "outcome": "rows"})
        own.append({"model": model, "own": [{"category": "missing rule", "construct": "fn:<udf>"}]})
    (tmp_path / "index-core.json").write_text(json.dumps(entries))
    (tmp_path / "index-edge.json").write_text("[]")
    census: Path = tmp_path / "census.json"
    census.write_text(json.dumps({"blockers": {"files": own}}))
    converted_names: list[str] = []
    compared: dict[str, object] = {}

    def convert_function(function: ProjectFunction) -> FunctionMacro | tuple[str, ...]:
        converted_names.append(function.name)
        return FunctionMacro(name=function.name, parameters=("sku",), body=function.body)

    def compare(
        *, manifest: Path, comparisons: list[dict[str, object]]
    ) -> dict[str, dict[str, str]]:
        for comparison in comparisons:
            connection: duckdb.DuckDBPyConnection = duckdb.connect(str(comparison["database"]))
            sql: str = Path(str(comparison["sql"])).read_text()
            compared[str(comparison["id"])] = connection.execute(sql).fetchall()
            connection.close()
        return {key: {"id": key, "outcome": "matches", "detail": ""} for key in compared}

    out: Path = tmp_path / "snowflake.json"
    run_measure(
        argv=[
            "snowflake",
            "--results",
            str(results),
            "--compiled",
            str(compiled),
            "--census",
            str(census),
            "--converted",
            str(converted),
            "--functions",
            str(compiled / "functions"),
            "--project",
            "unused",
            "--sqlquack",
            "unused",
            "--library",
            "unused",
            "--work",
            str(tmp_path / "work"),
            "--out",
            str(out),
        ],
        comparer=compare,
        function_converter=convert_function,
    )
    listed: object = json.loads(out.read_text())["results"]
    assert isinstance(listed, list)
    outcomes: dict[str, list[str]] = {
        str(item["model"]): [item["outcome"], *item["details"]] for item in listed
    }
    assert (converted_names, compared, outcomes) == (
        test_case.expected_converted,
        test_case.expected_compared,
        test_case.expected_outcomes,
    )
