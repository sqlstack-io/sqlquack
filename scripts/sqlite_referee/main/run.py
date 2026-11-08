"""Record SQLite 3.54.1 receipts for case files, compare two runs, or check one run."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.sqlite_referee._helpers.run import check, compare, read_cases, record
from scripts.sqlite_referee.constants import (
    COMMAND_CHECK,
    COMMAND_COMPARE,
    COMMAND_RUN,
    SHOWN_DIFFERENCES,
)
from scripts.sqlite_referee.types import Row


def run_referee(argv: list[str] | None = None) -> int:
    """`run --cache DIR --cases FILE... --out FILE --private DIR`, `compare A B` or `check`."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    commands: argparse._SubParsersAction = parser.add_subparsers(dest="command", required=True)
    run: argparse.ArgumentParser = commands.add_parser(COMMAND_RUN)
    run.add_argument("--cache", type=Path, required=True)
    run.add_argument("--cases", type=Path, nargs="+", required=True)
    run.add_argument("--out", type=Path, required=True)
    run.add_argument("--private", type=Path, required=True)
    pair: argparse.ArgumentParser = commands.add_parser(COMMAND_COMPARE)
    pair.add_argument("first", type=Path)
    pair.add_argument("second", type=Path)
    judged: argparse.ArgumentParser = commands.add_parser(COMMAND_CHECK)
    judged.add_argument("--cases", type=Path, nargs="+", required=True)
    judged.add_argument("--receipts", type=Path, required=True)
    arguments: argparse.Namespace = parser.parse_args(argv)
    if arguments.command == COMMAND_COMPARE:
        differ: list[str] = compare(first=arguments.first, second=arguments.second)
        print("\n".join(differ[:SHOWN_DIFFERENCES]), file=sys.stderr)
        return 1 if differ else 0
    if arguments.command == COMMAND_CHECK:
        result: Row = check(cases=read_cases(arguments.cases), receipts=arguments.receipts)
        print(json.dumps(result, indent=2, sort_keys=True))
        failed: bool = any(
            result[key] for key in ("missing", "stale", "false_accepts", "false_rejects")
        )
        return 1 if failed else 0
    report: Row = record(
        cases=read_cases(arguments.cases),
        out=arguments.out,
        private=arguments.private,
        cache=arguments.cache,
    )
    print(json.dumps(report, sort_keys=True))
    return 0
