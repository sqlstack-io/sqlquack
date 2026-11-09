"""Reading changes and blobs from Git."""

from __future__ import annotations

import subprocess
from pathlib import Path

from scripts.evidence_guard.constants import GIT_FIELD_SEPARATOR, RESULTS_PATHSPECS
from scripts.evidence_guard.models import ChangedPath


def changed_results(*, repository: Path, base: str, head: str) -> list[ChangedPath]:
    """Every change between the merge base of `base` and `head` under the results pathspecs."""
    output: str = subprocess.run(
        (
            "git",
            "diff",
            "--name-status",
            "--no-renames",
            f"{base}...{head}",
            "--",
            *RESULTS_PATHSPECS,
        ),
        cwd=repository,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    changes: list[ChangedPath] = []
    for line in output.splitlines():
        status, _, path = line.partition(GIT_FIELD_SEPARATOR)
        changes.append(ChangedPath(status=status, path=path))
    return changes


def blob(*, repository: Path, head: str, path: str) -> bytes | None:
    """The bytes of `path` at `head`, absent when it does not exist there."""
    result: subprocess.CompletedProcess[bytes] = subprocess.run(
        ("git", "show", f"{head}:{path}"), cwd=repository, check=False, capture_output=True
    )
    return result.stdout if result.returncode == 0 else None
