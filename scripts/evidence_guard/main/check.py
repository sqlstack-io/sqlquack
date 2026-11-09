"""Append-only evidence guard entry point."""

from __future__ import annotations

import argparse
import re
import sys
from collections.abc import Callable
from pathlib import Path, PurePosixPath

from scripts.evidence_guard._helpers.git import changed_results
from scripts.evidence_guard._helpers.metadata import added_result_problems
from scripts.evidence_guard._helpers.referee import added_referee_problems
from scripts.evidence_guard._helpers.syntax_receipts import added_syntax_receipt_problems
from scripts.evidence_guard.constants import (
    ADDED_STATUS,
    REFEREE_DIRECTORY,
    SYNTAX_RECEIPTS_PATTERN,
    UNGUARDED_FILES,
)
from scripts.evidence_guard.models import ChangedPath


def run_evidence_guard(argv: list[str] | None = None) -> int:
    """Reject modified, deleted or invalid recorded results between two revisions."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    arguments: argparse.Namespace = parser.parse_args(argv)
    changes: list[ChangedPath] = [
        change
        for change in changed_results(
            repository=arguments.repository, base=arguments.base, head=arguments.head
        )
        if PurePosixPath(change.path).name not in UNGUARDED_FILES
    ]
    problems: list[str] = []
    for change in changes:
        if change.status != ADDED_STATUS:
            problems.append(
                f"{change.path}: recorded results are append-only (status {change.status})"
            )
            continue
        checker: Callable[..., list[str]] = (
            added_referee_problems
            if change.path.startswith(REFEREE_DIRECTORY)
            else added_syntax_receipt_problems
            if re.match(SYNTAX_RECEIPTS_PATTERN, change.path)
            else added_result_problems
        )
        problems.extend(
            checker(repository=arguments.repository, head=arguments.head, path=change.path)
        )
    for problem in problems:
        print(f"evidence guard: {problem}", file=sys.stderr)
    added: int = sum(change.status == ADDED_STATUS for change in changes)
    print(
        f"evidence guard: {'FAILED' if problems else 'passed'}, {added} added result files, "
        f"{len(problems)} problems",
        file=sys.stderr,
    )
    return 1 if problems else 0
