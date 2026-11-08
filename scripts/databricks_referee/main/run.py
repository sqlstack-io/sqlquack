"""Record Databricks SQL 2026.44 receipts for case files, or compare two runs."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.databricks_referee._helpers.run import compare, read_cases, record
from scripts.databricks_referee._helpers.sequential import (
    record_sequential,
    recover,
    refinish_sequential,
    resume_sequential,
)
from scripts.databricks_referee.constants import (
    COMMAND_COMPARE,
    COMMAND_RECOVER,
    COMMAND_REFINISH,
    COMMAND_RESUME,
    COMMAND_RUN,
    SHOWN_DIFFERENCES,
)
from scripts.databricks_referee.types import Row


def run_referee(argv: list[str] | None = None) -> int:
    """`run --cases FILE... --out DIR --private DIR [--shuffle N] [--select IDS] [--sequential]`
    or `compare`, or `recover --journal FILE --private DIR`, or `resume --journal FILE` with the
    stopped sequential run's own `run` arguments. `--sequential` runs the cases one at a
    time, held cases included, restoring the account state after each; `recover` cleans up after a
    sequential run that stopped or froze, from its journal, and fails when anything is left;
    `resume` recovers it and continues it only when its cases, order, seed, source and account
    state match; `refinish --journal FILE`, with the same arguments, writes a finished sequential
    run's receipts with its failures judged again by the current reviewed rows."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    commands: argparse._SubParsersAction = parser.add_subparsers(dest="command", required=True)
    run: argparse.ArgumentParser = commands.add_parser(COMMAND_RUN)
    again: argparse.ArgumentParser = commands.add_parser(COMMAND_RESUME)
    again.add_argument("--journal", type=Path, required=True)
    redo: argparse.ArgumentParser = commands.add_parser(COMMAND_REFINISH)
    redo.add_argument("--journal", type=Path, required=True)
    for each in (run, again, redo):
        each.add_argument("--cases", type=Path, nargs="+", required=True)
        each.add_argument("--out", type=Path, required=True)
        each.add_argument("--private", type=Path, required=True)
        each.add_argument("--shuffle", type=int, default=None)
        each.add_argument("--only", nargs="*", default=None)
        each.add_argument("--select", type=Path, default=None)
    run.add_argument("--sequential", action="store_true")
    pair: argparse.ArgumentParser = commands.add_parser(COMMAND_COMPARE)
    pair.add_argument("first", type=Path)
    pair.add_argument("second", type=Path)
    back: argparse.ArgumentParser = commands.add_parser(COMMAND_RECOVER)
    back.add_argument("--journal", type=Path, required=True)
    back.add_argument("--private", type=Path, required=True)
    arguments: argparse.Namespace = parser.parse_args(argv)
    if arguments.command == COMMAND_RECOVER:
        report: Row = recover(path=arguments.journal, private=arguments.private)
        print(json.dumps(report, sort_keys=True))
        return 1 if any(dict(report["left"]).values()) else 0  # type: ignore[call-overload]
    if arguments.command == COMMAND_COMPARE:
        differ: list[str] = compare(first=arguments.first, second=arguments.second)
        print("\n".join(differ[:SHOWN_DIFFERENCES]), file=sys.stderr)
        return 1 if differ else 0
    cases: list[Row] = read_cases(arguments.cases)
    chosen: set[str] = set(arguments.only or [])
    if arguments.select is not None:
        chosen |= {
            line.strip() for line in arguments.select.read_text().splitlines() if line.strip()
        }
    if chosen:
        cases = [case for case in cases if str(case["case_id"]) in chosen]
    if arguments.command in (COMMAND_RESUME, COMMAND_REFINISH):
        report = (
            resume_sequential if arguments.command == COMMAND_RESUME else refinish_sequential
        )(
            path=arguments.journal,
            cases=cases,
            out=arguments.out,
            private=arguments.private,
            seed=arguments.shuffle,
        )
        print(json.dumps(report, sort_keys=True))
        return 0
    report = (record_sequential if arguments.sequential else record)(
        cases=cases, out=arguments.out, private=arguments.private, seed=arguments.shuffle
    )
    print(json.dumps(report, sort_keys=True))
    return 0
