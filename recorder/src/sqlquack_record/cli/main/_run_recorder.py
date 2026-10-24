"""The `sqlquack-record` command."""

from __future__ import annotations

import argparse
import os
import sys
from datetime import UTC, datetime
from pathlib import Path

from sqlquack_record.pending.main.load_pending import load_pending
from sqlquack_record.recording.constants import (
    CONNECTION_ENVIRONMENT,
    CONNECTION_OVERRIDES,
    DEFAULT_BATCH_SIZE,
    DEFAULT_PENDING_DIR,
    DEFAULT_RESULTS_DIR,
    WAREHOUSE_ENVIRONMENT,
)
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.main.record_pending import record_pending
from sqlquack_record.recording.models import PendingCase, RecordSummary
from sqlquack_record.snowflake.main.open_snowflake_sessions import snowflake_sessions
from sqlquack_record.snowflake.models import ConnectionSettings


def main(argv: list[str] | None = None) -> int:
    """Record every pending case on Snowflake; results are written once and never replaced."""
    arguments: argparse.Namespace = _parser().parse_args(argv)
    try:
        cases: list[PendingCase] = load_pending(arguments.pending)
        print(
            f"sqlquack-record: {len(cases)} pending cases in {arguments.pending}", file=sys.stderr
        )
        if arguments.dry_run or not cases:
            for case in cases:
                print(f"{case.case_hash}  {case.case_reference}")
            return 0
        settings: ConnectionSettings = ConnectionSettings(
            connection_name=arguments.connection,
            warehouse=arguments.warehouse,
            overrides=tuple(
                (argument, os.environ[variable])
                for variable, argument in sorted(CONNECTION_OVERRIDES.items())
                if os.environ.get(variable)
            ),
        )
        print("sqlquack-record: connecting to Snowflake and recording…", file=sys.stderr)
        summary: RecordSummary = record_pending(
            cases=cases,
            results=arguments.results,
            open_session=snowflake_sessions(settings),
            batch_size=arguments.batch_size,
            clock=lambda: datetime.now(UTC).isoformat(timespec="seconds").replace("+00:00", "Z"),
        )
    except RecorderError as error:
        print(f"sqlquack-record: failed: {error}", file=sys.stderr)
        return 1
    print(
        f"sqlquack-record: done, recorded {summary.recorded} results "
        f"({summary.failures} warehouse SQL errors) into {arguments.results}",
        file=sys.stderr,
    )
    return 0


def _parser() -> argparse.ArgumentParser:
    parser: argparse.ArgumentParser = argparse.ArgumentParser(
        prog="sqlquack-record",
        description="Record Snowflake results for SQLQuack's pending cases as Arrow IPC files.",
    )
    parser.add_argument("--pending", type=Path, default=Path(DEFAULT_PENDING_DIR))
    parser.add_argument("--results", type=Path, default=Path(DEFAULT_RESULTS_DIR))
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
    return parser
