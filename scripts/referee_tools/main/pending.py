"""Referee pending entry point: write a pending file for every unrecorded referee case."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.referee_tools._helpers.loading import referee_cases
from scripts.referee_tools._helpers.pending import write_pending
from scripts.referee_tools.constants import (
    EVIDENCE_DIRECTORY,
    JSON_SUFFIX,
    PENDING_DIRECTORY,
    RESULTS_DIRECTORY,
)
from scripts.referee_tools.models import RefereeCase


def run_pending(argv: list[str] | None = None) -> int:
    """Write pending referee files for the selected files' unrecorded cases, at most --limit."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument(
        "--select",
        action="append",
        default=[],
        help="a glob of referee files below referee/, such as generated/*.json; repeatable",
    )
    parser.add_argument("--limit", type=int, default=None, help="the most cases to write")
    parser.add_argument(
        "--catalogue", action="store_true", help="also request the built-in function catalogue"
    )
    arguments: argparse.Namespace = parser.parse_args(argv)
    repository: Path = arguments.repository
    print("referee pending: reading referee cases…", file=sys.stderr)
    results: Path = repository / EVIDENCE_DIRECTORY / RESULTS_DIRECTORY
    patterns: tuple[str, ...] = tuple(arguments.select) or ("**/*.json",)
    unrecorded: dict[str, RefereeCase] = {}
    for case in referee_cases(repository=repository, patterns=patterns):
        if not (results / f"{case.case_hash}{JSON_SUFFIX}").exists():
            unrecorded.setdefault(case.case_hash, case)
    chosen: list[RefereeCase] = list(unrecorded.values())[: arguments.limit]
    written, removed, environments = write_pending(
        directory=repository / PENDING_DIRECTORY, cases=chosen, catalogue=arguments.catalogue
    )
    print(
        f"referee pending: done, {written} cases in {environments} environments pending "
        f"({len(unrecorded)} unrecorded), {removed} stale files removed"
        f"{', catalogue requested' if arguments.catalogue else ''}",
        file=sys.stderr,
    )
    return 0
