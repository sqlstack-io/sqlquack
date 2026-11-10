"""`sqlquack.convert` reports what `sqlquack convert --json` reports."""

from __future__ import annotations

from pathlib import Path

import pytest

import sqlquack
from sqlquack import Conversion, ConvertOptions, Diagnostic
from tests.unit.python.src.sqlquack.api.main._test_types import (
    CommandLineTestCase,
    DeepChainTestCase,
    FailureTestCase,
    ProjectTestCase,
    TypedDiagnosticTestCase,
)
from tests.unit.python.src.sqlquack.api.main.helpers import (
    SCHEMA,
    command_report,
    source_project,
)

CONFIG: str = str(Path(__file__).resolve().parents[7] / "sqlquack.toml")
HASHED: str = "SELECT HASH(order_id) AS h FROM orders"
DATED: str = "SELECT CURRENT_DATE() AS d"


@pytest.mark.parametrize(
    "test_case",
    (
        CommandLineTestCase(
            description="a converted query",
            sql="SELECT order_id, price * 2 AS doubled FROM orders",
            arguments=("--no-config",),
            options=ConvertOptions(no_config=True, schema=SCHEMA),
            expected_converted=True,
        ),
        CommandLineTestCase(
            description="a converted query on one line",
            sql="SELECT order_id, price * 2 AS doubled FROM orders",
            arguments=("--no-config", "--compact"),
            options=ConvertOptions(no_config=True, schema=SCHEMA, compact=True),
            expected_converted=True,
        ),
        CommandLineTestCase(
            description="no --config and no sqlquack.toml: the defaults",
            sql="SELECT order_id, price * 2 AS doubled FROM orders",
            arguments=(),
            options=ConvertOptions(schema=SCHEMA),
            expected_converted=True,
        ),
        CommandLineTestCase(
            description="a query refused for an undeclared relation",
            sql="SELECT a FROM customers",
            arguments=("--no-config",),
            options=ConvertOptions(no_config=True, schema=SCHEMA),
            expected_converted=False,
        ),
        CommandLineTestCase(
            description="a query refused under a session no profile covers",
            sql="SELECT order_id FROM orders",
            arguments=("--no-config", "--session", "WEEK_START=3"),
            options=ConvertOptions(no_config=True, schema=SCHEMA, session={"WEEK_START": "3"}),
            expected_converted=False,
        ),
        CommandLineTestCase(
            description="HASH is refused by default, as its value is never Snowflake's",
            sql=HASHED,
            arguments=("--config", CONFIG),
            options=ConvertOptions(config=CONFIG, schema=SCHEMA),
            expected_converted=False,
        ),
        CommandLineTestCase(
            description="CURRENT_DATE relies on value differences the report names",
            sql=DATED,
            arguments=("--config", CONFIG),
            options=ConvertOptions(config=CONFIG, schema=SCHEMA),
            expected_converted=True,
        ),
        CommandLineTestCase(
            description="strict mode refuses CURRENT_DATE whatever the configuration allows",
            sql=DATED,
            arguments=("--config", CONFIG, "--strict"),
            options=ConvertOptions(config=CONFIG, schema=SCHEMA, strict=True),
            expected_converted=False,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_the_same_options_when_converting_then_the_report_is_the_command_lines(
    test_case: CommandLineTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    project: Path = source_project(tmp_path / "project")
    elsewhere: Path = tmp_path / "elsewhere"
    elsewhere.mkdir()
    monkeypatch.chdir(elsewhere)
    conversion: Conversion = sqlquack.convert(sql=test_case.sql, options=test_case.options)
    assert conversion.report == command_report(test_case.sql, test_case.arguments, project)
    assert conversion.converted is test_case.expected_converted
    assert len(conversion.diagnostics) == len(conversion.report["refusals"])
    assert conversion.divergence_failures == conversion.report["divergence_failures"]


@pytest.mark.parametrize(
    "test_case",
    (
        TypedDiagnosticTestCase(
            description="an undeclared relation is SQ0004 at the relation's name",
            sql="SELECT a FROM orders",
            options=ConvertOptions(no_config=True),
            expected_identity=("SQ0004", "unresolved_name", "error"),
            expected_span=sqlquack.Span(
                start=14, end=20, line=1, column=15, end_line=1, end_column=21
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_refusal_when_converting_then_the_diagnostic_is_typed(
    test_case: TypedDiagnosticTestCase,
) -> None:
    conversion: Conversion = sqlquack.convert(sql=test_case.sql, options=test_case.options)
    found: Diagnostic = conversion.diagnostics[0]
    assert (found.code, found.name, found.severity) == test_case.expected_identity
    assert found.span == test_case.expected_span
    assert found.help is not None
    assert conversion.sql == ""


@pytest.mark.parametrize(
    "test_case",
    (
        FailureTestCase(
            description="a configuration file that does not exist, as on the command line",
            sql="SELECT 1 AS one",
            options=ConvertOptions(config="missing.toml"),
            expected_message="no configuration file at missing.toml",
        ),
        FailureTestCase(
            description="input holding no statement",
            sql="-- nothing here",
            options=ConvertOptions(no_config=True),
            expected_message="the input holds no SQL statement",
        ),
        FailureTestCase(
            description="a schema option that is not a schema",
            sql="SELECT 1",
            options=ConvertOptions(no_config=True, schema={"ORDERS": "order_id"}),
            expected_message="the schema option is not a schema",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_input_the_command_line_exits_2_on_when_converting_then_sqlquack_error_is_raised(
    test_case: FailureTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.chdir(tmp_path)
    with pytest.raises(sqlquack.SqlQuackError, match=test_case.expected_message):
        sqlquack.convert(sql=test_case.sql, options=test_case.options)


@pytest.mark.parametrize(
    "test_case",
    (
        DeepChainTestCase(
            description="a long chain of additions is refused, not a crash of the interpreter",
            sql="SELECT " + " + ".join(["x"] * 3_000) + " AS y FROM (SELECT 1 AS x)",
            expected_converted=False,
            expected_codes=("SQ0006",),
        ),
        DeepChainTestCase(
            description="a long chain of || is refused, not a crash of the interpreter",
            sql="SELECT " + " || ".join(["s"] * 3_000) + " AS y FROM (SELECT 'a' AS s)",
            expected_converted=False,
            expected_codes=("SQ0006",),
        ),
        DeepChainTestCase(
            description="a || chain within the depth limit converts",
            sql="SELECT " + " || ".join(["s"] * 400) + " AS y FROM (SELECT 'a' AS s)",
            expected_converted=True,
            expected_codes=(),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_long_flat_chain_when_converting_then_the_interpreter_survives(
    test_case: DeepChainTestCase,
) -> None:
    conversion: Conversion = sqlquack.convert(
        sql=test_case.sql, options=ConvertOptions(no_config=True)
    )
    assert conversion.converted is test_case.expected_converted
    assert tuple(found.code for found in conversion.diagnostics) == test_case.expected_codes


@pytest.mark.parametrize(
    "test_case",
    (
        ProjectTestCase(
            description="the project's sources type the query and its session applies",
            session={},
            arguments=(),
            expected_notes=1,
        ),
        ProjectTestCase(
            description="a session option replaces the project's value for the call",
            session={"TIMEZONE": "UTC"},
            arguments=("--session", "TIMEZONE=UTC"),
            expected_notes=0,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_project_folder_when_converting_then_its_sources_and_settings_apply(
    test_case: ProjectTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    project: Path = tmp_path / "project"
    (project / "sources").mkdir(parents=True)
    (project / "sqlquack.toml").write_text(
        Path(CONFIG).read_text() + '\n[project.session]\nTIMEZONE = "America/New_York"\n'
    )
    (project / "sources" / "raw.yml").write_text(
        "sources:\n  - name: orders\n    columns:\n      - name: order_id\n"
        "        type: NUMBER(38,0)\n"
    )
    monkeypatch.chdir(tmp_path)
    sql: str = "SELECT order_id + 1 AS next_id FROM orders"
    options: ConvertOptions = ConvertOptions(project=project, session=test_case.session)
    conversion: Conversion = sqlquack.convert(sql=sql, options=options)
    assert conversion.converted is True
    assert conversion.report == command_report(sql, test_case.arguments, project)
    assert len(conversion.notes) == test_case.expected_notes
