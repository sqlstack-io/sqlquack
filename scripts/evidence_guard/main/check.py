"""Append-only evidence guard entry point."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path, PurePosixPath

from scripts.evidence_guard._helpers.git import changed_results
from scripts.evidence_guard._helpers.metadata import added_result_problems
from scripts.evidence_guard.constants import UNGUARDED_FILES
from scripts.evidence_guard.models import ChangedPath


def run_evidence_guard(argv: list[str] | None = None) -> int:
    """Reject modified, deleted or invalid recorded results between two revisions."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    arguments: argparse.Namespace = parser.parse_args(argv)
    changes: list[ChangedPath] = changed_results(
        arguments.repository, arguments.base, arguments.head
    )
    problems: list[str] = []
    for change in changes:
        if PurePosixPath(change.path).name in UNGUARDED_FILES:
            continue
        if change.status != "A":
            problems.append(
                f"{change.path}: recorded results are append-only (status {change.status})"
            )
            continue
        problems.extend(added_result_problems(arguments.repository, arguments.head, change.path))
    for problem in problems:
        print(f"evidence guard: {problem}", file=sys.stderr)
    added: int = sum(change.status == "A" for change in changes)
    print(
        f"evidence guard: {'FAILED' if problems else 'passed'}, {added} added result files, "
        f"{len(problems)} problems",
        file=sys.stderr,
    )
    return 1 if problems else 0
