"""The `sqlquack-record` command."""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

from sqlquack_record.cli._helpers.settings import connection_settings, utc_now
from sqlquack_record.pending.main.evidence_directories import evidence_directories
from sqlquack_record.pending.main.load_pending import load_pending
from sqlquack_record.recording.constants import (
    CONNECTION_ENVIRONMENT,
    DEFAULT_BATCH_SIZE,
    WAREHOUSE_ENVIRONMENT,
)
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.main.record_pending import record_pending
from sqlquack_record.recording.main.record_session_defaults import record_session_defaults
from sqlquack_record.recording.models import PendingCase, RecordSummary
from sqlquack_record.snowflake.main.open_snowflake_sessions import snowflake_sessions
from sqlquack_record.snowflake.models import ConnectionSettings


def main(argv: list[str] | None = None) -> int:
    """Record every pending case on Snowflake; results are written once and never replaced."""
    arguments: argparse.Namespace = _parser().parse_args(argv)
    if arguments.session_defaults is not None:
        return _session_defaults(arguments)
    try:
        pending, results = evidence_directories(
            pending=arguments.pending, results=arguments.results, working_directory=Path.cwd()
        )
        cases: list[PendingCase] = load_pending(pending)
        print(f"sqlquack-record: {len(cases)} pending cases in {pending}", file=sys.stderr)
        if arguments.dry_run or not cases:
            for case in cases:
                print(f"{case.case_hash}  {case.case_reference}")
            return 0
        settings: ConnectionSettings = connection_settings(arguments)
        print("sqlquack-record: connecting to Snowflake and recording…", file=sys.stderr)
        summary: RecordSummary = record_pending(
            cases=cases,
            results=results,
            open_session=snowflake_sessions(settings),
            batch_size=arguments.batch_size,
            clock=utc_now,
        )
    except RecorderError as error:
        print(f"sqlquack-record: failed: {error}", file=sys.stderr)
        return 1
    for reference, reason in summary.refused:
        print(f"sqlquack-record: refused {reference}: {reason}", file=sys.stderr)
    status: str = "FAILED" if summary.refused else "done"
    print(
        f"sqlquack-record: {status}, recorded {summary.recorded} results "
        f"({summary.failures} warehouse SQL errors) into {results}; "
        f"{len(summary.refused)} cases refused and left pending",
        file=sys.stderr,
    )
    return 1 if summary.refused else 0


def _parser() -> argparse.ArgumentParser:
    parser: argparse.ArgumentParser = argparse.ArgumentParser(
        prog="sqlquack-record",
        description="Record Snowflake results for SQLQuack's pending cases as Arrow IPC files.",
    )
    parser.add_argument(
        "--pending",
        type=Path,
        help="default: evidence/pending in the repository",
    )
    parser.add_argument("--results", type=Path, help="default: evidence/results in the repository")
    parser.add_argument(
        "--connection",
        default=os.environ.get(CONNECTION_ENVIRONMENT),
        help="a connection name from ~/.snowflake/connections.toml",
    )
    parser.add_argument(
        "--warehouse",
        default=os.environ.get(WAREHOUSE_ENVIRONMENT),
        help="the warehouse to use instead of the connection's",
    )
    parser.add_argument("--batch-size", type=int, default=DEFAULT_BATCH_SIZE)
    parser.add_argument("--dry-run", action="store_true", help="list pending cases and stop")
    parser.add_argument(
        "--session-defaults",
        type=Path,
        help="write the account's default session parameters to this file and stop",
    )
    return parser


def _session_defaults(arguments: argparse.Namespace) -> int:
    """Read the defaults of a session that sets no parameters."""
    print("sqlquack-record: reading the account's default session parameters…", file=sys.stderr)
    try:
        pending, _ = evidence_directories(
            pending=arguments.pending, results=arguments.results, working_directory=Path.cwd()
        )
        names: frozenset[str] = frozenset().union(
            *(dict(case.recording_parameters()) for case in load_pending(pending))
        )
        if not names:
            raise RecorderError("no pending case names a parameter; run `sqlquack pending` first")
        count: int = record_session_defaults(
            out=arguments.session_defaults,
            names=names,
            open_session=snowflake_sessions(connection_settings(arguments)),
            clock=utc_now,
        )
    except RecorderError as error:
        print(f"sqlquack-record: failed: {error}", file=sys.stderr)
        return 1
    print(
        f"sqlquack-record: done, {count} parameters written to {arguments.session_defaults}",
        file=sys.stderr,
    )
    return 0
