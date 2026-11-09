"""The referee part of a `sqlquack-record` run."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from sqlquack_record.cli._helpers.settings import connection_settings, utc_now
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.referee.constants import REFEREE_DIRECTORY
from sqlquack_record.referee.main.record_referee import record_referee
from sqlquack_record.referee.models import RefereePending, RefereeSummary
from sqlquack_record.snowflake.main.open_referee_sessions import referee_sessions


def dry_run_referee(referee: RefereePending | None) -> int:
    """List the pending referee cases and stop."""
    if referee is None:
        return 0
    print(
        f"sqlquack-record: {len(referee.cases)} pending referee cases in "
        f"{len(referee.environments)} environments; catalogue "
        f"{'requested' if referee.catalogue else 'not requested'}",
        file=sys.stderr,
    )
    for case in referee.cases:
        print(f"{case.case_hash}  {case.case_reference}")
    return 0


def run_referee(
    *, arguments: argparse.Namespace, referee: RefereePending | None, results: Path
) -> int:
    """Compile the pending referee cases into the referee evidence beside `results`."""
    if referee is None or not (referee.cases or referee.catalogue):
        return 0
    directory: Path = results.parent / REFEREE_DIRECTORY
    print(
        f"sqlquack-record: compiling {len(referee.cases)} referee cases on Snowflake…",
        file=sys.stderr,
    )
    try:
        summary: RefereeSummary = record_referee(
            pending=referee,
            directory=directory,
            open_session=referee_sessions(connection_settings(arguments)),
            clock=utc_now,
        )
    except RecorderError as error:
        print(f"sqlquack-record: referee failed: {error}", file=sys.stderr)
        return 1
    for reference, reason in summary.refused:
        print(f"sqlquack-record: refused referee {reference}: {reason}", file=sys.stderr)
    catalogue: str = (
        "" if summary.catalogue is None else f"; catalogue {summary.catalogue[:12]} written"
    )
    print(
        f"sqlquack-record: referee {'FAILED' if summary.refused else 'done'}, recorded "
        f"{summary.recorded} outcomes ({summary.accepted} accepted) into {directory}; "
        f"{len(summary.refused)} referee cases refused and left pending{catalogue}",
        file=sys.stderr,
    )
    return 1 if summary.refused else 0
