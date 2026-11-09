"""DuckShop measures: the census project from a referee environment, and tests on DuckDB."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from scripts.duckshop_measure._helpers.functions import project_macros
from scripts.duckshop_measure._helpers.measures import (
    snowflake_results,
    unit_test_outcomes,
    write_project,
)
from scripts.duckshop_measure._helpers.report import (
    snowflake_report,
    snowflake_summary,
    unit_tests_report,
    unit_tests_summary,
)
from scripts.duckshop_measure._helpers.running import binary_converter, duckdb_rows
from scripts.duckshop_measure._helpers.snowflake import policy_value_divergences, run_comparisons
from scripts.duckshop_measure.constants import PROJECT_COMMAND, SNOWFLAKE_COMMAND, TESTS_COMMAND
from scripts.duckshop_measure.models import ModelResult, SnowflakeInputs, UnitTestOutcome
from scripts.duckshop_measure.types import Comparer, Converter, FunctionConverter


def _measure_parser() -> argparse.ArgumentParser:
    """The `project`, `tests` and `snowflake` subcommands and their arguments."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    commands: argparse._SubParsersAction[argparse.ArgumentParser] = parser.add_subparsers(
        dest="command", required=True
    )
    project: argparse.ArgumentParser = commands.add_parser(
        PROJECT_COMMAND, help="a census project folder from a referee environment"
    )
    project.add_argument("--environment", type=Path, required=True)
    project.add_argument(
        "--out", type=Path, required=True, help="the project folder its sources go to"
    )
    project.add_argument(
        "--recorded", type=Path, help="recorded results whose relations the project types instead"
    )
    project.add_argument(
        "--functions", type=Path, help="compiled project functions the project declares"
    )
    tests: argparse.ArgumentParser = commands.add_parser(
        TESTS_COMMAND, help="compiled unit tests converted and run on DuckDB"
    )
    tests.add_argument("--compiled", type=Path, required=True, help="a project's target/compiled")
    tests.add_argument("--sqlquack", type=Path, required=True)
    tests.add_argument("--library", type=Path, required=True, help="libduckdb.so's directory")
    tests.add_argument("--policy", type=Path, default=Path("sqlquack.toml"))
    tests.add_argument("--out", type=Path, required=True)
    snowflake: argparse.ArgumentParser = commands.add_parser(
        SNOWFLAKE_COMMAND, help="converted models on DuckDB over recorded inputs, against Snowflake"
    )
    snowflake.add_argument("--results", type=Path, required=True, help="recorded DuckShop results")
    snowflake.add_argument(
        "--compiled", type=Path, required=True, help="a project's target/compiled"
    )
    snowflake.add_argument(
        "--census", type=Path, required=True, help="census --detailed --json output"
    )
    snowflake.add_argument(
        "--converted", type=Path, required=True, help="SQLQUACK_CONVERTED_DIR of the strict census"
    )
    snowflake.add_argument(
        "--lenient-converted",
        type=Path,
        help="with a strict census: SQLQUACK_CONVERTED_DIR of a census under the default policy",
    )
    snowflake.add_argument("--guards", type=Path, help="SQLQUACK_GUARDS_DIR of the census")
    snowflake.add_argument(
        "--functions", type=Path, help="compiled project functions, created in DuckDB before models"
    )
    snowflake.add_argument(
        "--project",
        type=Path,
        help="the census project folder declaring the functions (with --functions)",
    )
    snowflake.add_argument("--sqlquack", type=Path, required=True)
    snowflake.add_argument("--library", type=Path, required=True, help="libduckdb.so's directory")
    snowflake.add_argument("--policy", type=Path, default=Path("sqlquack.toml"))
    snowflake.add_argument("--work", type=Path, required=True, help="inputs databases and manifest")
    snowflake.add_argument("--out", type=Path, required=True)
    return parser


def _measure_snowflake(
    *,
    arguments: argparse.Namespace,
    comparer: Comparer | None,
    function_converter: FunctionConverter | None,
) -> None:
    """Write the Snowflake comparison report and print its summary."""

    def compare(
        *, manifest: Path, comparisons: list[dict[str, object]]
    ) -> dict[str, dict[str, str]]:
        return run_comparisons(
            binary=arguments.sqlquack,
            library=arguments.library,
            policy=arguments.policy,
            manifest=manifest,
            comparisons=comparisons,
        )

    arguments.work.mkdir(parents=True, exist_ok=True)
    macros, python, refused = project_macros(
        arguments=arguments, function_converter=function_converter
    )
    inputs: SnowflakeInputs = SnowflakeInputs(
        results=arguments.results,
        compiled=arguments.compiled,
        census=arguments.census,
        converted=arguments.converted,
        lenient_converted=arguments.lenient_converted,
        guards=arguments.guards,
        value_divergences=policy_value_divergences(arguments.policy),
        work=arguments.work,
        macros=macros,
        python_functions=python,
        unavailable_functions=tuple(
            (name, f"sqlquack function refused: {'; '.join(why)}")
            for name, why in sorted(refused.items())
        ),
    )
    models: list[ModelResult] = snowflake_results(inputs=inputs, run=comparer or compare)
    arguments.out.write_text(json.dumps(snowflake_report(models), indent=1) + "\n")
    print(snowflake_summary(models))


def run_measure(
    *,
    argv: list[str] | None = None,
    converter: Converter | None = None,
    comparer: Comparer | None = None,
    function_converter: FunctionConverter | None = None,
) -> int:
    """Run one DuckShop measure subcommand; 0 once its report is written."""
    arguments: argparse.Namespace = _measure_parser().parse_args(argv)
    if arguments.command == PROJECT_COMMAND:
        write_project(
            environment=arguments.environment,
            out=arguments.out,
            recorded=arguments.recorded,
            functions=arguments.functions,
        )
        return 0
    if arguments.command == SNOWFLAKE_COMMAND:
        _measure_snowflake(
            arguments=arguments, comparer=comparer, function_converter=function_converter
        )
        return 0
    convert: Converter = converter or binary_converter(
        binary=arguments.sqlquack, library=arguments.library, policy=arguments.policy
    )
    outcomes: list[UnitTestOutcome] = unit_test_outcomes(
        compiled=arguments.compiled, convert=convert, execute=duckdb_rows
    )
    arguments.out.write_text(json.dumps(unit_tests_report(outcomes), indent=1) + "\n")
    print(unit_tests_summary(outcomes))
    return 0
