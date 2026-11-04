"""DuckShop measures: the census schema from a referee environment, and unit tests on DuckDB."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from scripts.duckshop_measure._helpers.measures import (
    snowflake_results,
    unit_test_outcomes,
    write_schema,
)
from scripts.duckshop_measure._helpers.report import (
    snowflake_report,
    snowflake_summary,
    unit_tests_report,
    unit_tests_summary,
)
from scripts.duckshop_measure._helpers.running import binary_converter, duckdb_rows
from scripts.duckshop_measure._helpers.snowflake import policy_value_divergences, run_comparisons
from scripts.duckshop_measure.constants import SCHEMA_COMMAND, SNOWFLAKE_COMMAND, TESTS_COMMAND
from scripts.duckshop_measure.models import ModelResult, SnowflakeInputs, UnitTestOutcome
from scripts.duckshop_measure.types import Comparer, Converter


def _measure_parser() -> argparse.ArgumentParser:
    """The `schema`, `tests` and `snowflake` subcommands and their arguments."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    commands: argparse._SubParsersAction[argparse.ArgumentParser] = parser.add_subparsers(
        dest="command", required=True
    )
    schema: argparse.ArgumentParser = commands.add_parser(
        SCHEMA_COMMAND, help="a census schema from a referee environment"
    )
    schema.add_argument("--environment", type=Path, required=True)
    schema.add_argument("--out", type=Path, required=True)
    schema.add_argument(
        "--recorded", type=Path, help="recorded results whose relations the schema types instead"
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
        "--census", type=Path, required=True, help="census --json --names output"
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
    snowflake.add_argument("--sqlquack", type=Path, required=True)
    snowflake.add_argument("--library", type=Path, required=True, help="libduckdb.so's directory")
    snowflake.add_argument("--policy", type=Path, default=Path("sqlquack.toml"))
    snowflake.add_argument("--work", type=Path, required=True, help="inputs databases and manifest")
    snowflake.add_argument("--out", type=Path, required=True)
    return parser


def _measure_snowflake(*, arguments: argparse.Namespace, comparer: Comparer | None) -> None:
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
    inputs: SnowflakeInputs = SnowflakeInputs(
        results=arguments.results,
        compiled=arguments.compiled,
        census=arguments.census,
        converted=arguments.converted,
        lenient_converted=arguments.lenient_converted,
        guards=arguments.guards,
        value_divergences=policy_value_divergences(arguments.policy),
        work=arguments.work,
    )
    models: list[ModelResult] = snowflake_results(inputs=inputs, run=comparer or compare)
    arguments.out.write_text(json.dumps(snowflake_report(models), indent=1) + "\n")
    print(snowflake_summary(models))


def run_measure(
    *,
    argv: list[str] | None = None,
    converter: Converter | None = None,
    comparer: Comparer | None = None,
) -> int:
    """Run one DuckShop measure subcommand; 0 once its report is written."""
    arguments: argparse.Namespace = _measure_parser().parse_args(argv)
    if arguments.command == SCHEMA_COMMAND:
        write_schema(
            environment=arguments.environment, out=arguments.out, recorded=arguments.recorded
        )
        return 0
    if arguments.command == SNOWFLAKE_COMMAND:
        _measure_snowflake(arguments=arguments, comparer=comparer)
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
