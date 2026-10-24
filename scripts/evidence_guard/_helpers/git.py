"""Reading changes and blobs from Git."""

from __future__ import annotations

import subprocess
from pathlib import Path

from scripts.evidence_guard.constants import RESULTS_DIR
from scripts.evidence_guard.models import ChangedPath


def changed_results(repository: Path, base: str, head: str) -> list[ChangedPath]:
    """Every change under the results directory between the merge base of `base` and `head`."""
    output: str = subprocess.run(
        ("git", "diff", "--name-status", "--no-renames", f"{base}...{head}", "--", RESULTS_DIR),
        cwd=repository,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [
        ChangedPath(status=line.split("\t", 1)[0], path=line.split("\t", 1)[1])
        for line in output.splitlines()
        if "\t" in line
    ]


def blob(repository: Path, head: str, path: str) -> bytes | None:
    """The bytes of `path` at `head`, absent when it does not exist there."""
    result: subprocess.CompletedProcess[bytes] = subprocess.run(
        ("git", "show", f"{head}:{path}"), cwd=repository, check=False, capture_output=True
    )
    return result.stdout if result.returncode == 0 else None
