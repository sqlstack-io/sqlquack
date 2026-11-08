"""Record BigQuery syntax receipts for case files, compare two runs, or probe one text."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.bigquery_referee._helpers.engine import environment_transport
from scripts.bigquery_referee._helpers.names import run_project
from scripts.bigquery_referee._helpers.refusals import write_refusals
from scripts.bigquery_referee._helpers.run import compare, probe, read_cases, record
from scripts.bigquery_referee.constants import (
    COMMAND_COMPARE,
    COMMAND_PROBE,
    COMMAND_REFUSALS,
    COMMAND_RUN,
    SHOWN_DIFFERENCES,
    WORKERS,
)
from scripts.bigquery_referee.models import RunPaths
from scripts.bigquery_referee.types import Row


def run_referee(argv: list[str] | None = None) -> int:
    """`run`, `compare`, `probe` or `refusals`; see the BigQuery root's README."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    commands: argparse._SubParsersAction = parser.add_subparsers(dest="command", required=True)
    run: argparse.ArgumentParser = commands.add_parser(COMMAND_RUN)
    run.add_argument("--cases", type=Path, nargs="+", required=True)
    run.add_argument("--out", type=Path, required=True)
    run.add_argument("--private", type=Path, required=True)
    run.add_argument("--reverse", action="store_true", help="judge the cases in reverse order")
    run.add_argument("--workers", type=int, default=WORKERS, help="parallel dry runs")
    pair: argparse.ArgumentParser = commands.add_parser(COMMAND_COMPARE)
    pair.add_argument("first", type=Path)
    pair.add_argument("second", type=Path)
    one: argparse.ArgumentParser = commands.add_parser(COMMAND_PROBE)
    one.add_argument("sql", help="case text, with the SQLQUACK_* tokens")
    one.add_argument("--family", default="query", help="the family whose policy judges it")
    table: argparse.ArgumentParser = commands.add_parser(COMMAND_REFUSALS)
    table.add_argument("grammar", type=Path, help="the pinned googlesql/parser/googlesql.tm")
    arguments: argparse.Namespace = parser.parse_args(argv)
    if arguments.command == COMMAND_REFUSALS:
        print(f"{write_refusals(arguments.grammar)} parser refusal classes")
        return 0
    if arguments.command == COMMAND_COMPARE:
        differ: list[str] = compare(first=arguments.first, second=arguments.second)
        print("\n".join(differ[:SHOWN_DIFFERENCES]), file=sys.stderr)
        return 1 if differ else 0
    if arguments.command == COMMAND_PROBE:
        found: Row = probe(
            sql=arguments.sql,
            family=arguments.family,
            transport=environment_transport(),
            project=run_project(),
        )
        print(json.dumps(found, indent=2, sort_keys=True))
        return 0
    cases: list[Row] = read_cases(arguments.cases)
    report: Row = record(
        cases=cases[::-1] if arguments.reverse else cases,
        paths=RunPaths(cases=tuple(arguments.cases), out=arguments.out, private=arguments.private),
        transport=environment_transport(),
        project=run_project(),
        workers=arguments.workers,
    )
    print(json.dumps(report, sort_keys=True))
    return 0
