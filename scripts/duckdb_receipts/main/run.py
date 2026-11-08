"""Record DuckDB 2.0.0 receipts for case files, or compare two runs."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.duckdb_receipts._helpers.run import compare, read_cases, record
from scripts.duckdb_receipts.constants import COMMAND_COMPARE, COMMAND_RUN, SHOWN_DIFFERENCES
from scripts.duckdb_receipts.types import Row


def run_referee(argv: list[str] | None = None) -> int:
    """`run --cases FILE... --out RECEIPTS --private DIR` or `compare RUN_A RUN_B`."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    commands: argparse._SubParsersAction = parser.add_subparsers(dest="command", required=True)
    run: argparse.ArgumentParser = commands.add_parser(COMMAND_RUN)
    run.add_argument("--cases", type=Path, nargs="+", required=True)
    run.add_argument("--out", type=Path, required=True)
    run.add_argument("--private", type=Path, required=True)
    pair: argparse.ArgumentParser = commands.add_parser(COMMAND_COMPARE)
    pair.add_argument("first", type=Path)
    pair.add_argument("second", type=Path)
    arguments: argparse.Namespace = parser.parse_args(argv)
    if arguments.command == COMMAND_COMPARE:
        differ: list[str] = compare(first=arguments.first, second=arguments.second)
        print("\n".join(differ[:SHOWN_DIFFERENCES]), file=sys.stderr)
        return 1 if differ else 0
    report: Row = record(
        cases=read_cases(arguments.cases), out=arguments.out, private=arguments.private
    )
    print(json.dumps(report, sort_keys=True))
    return 0
