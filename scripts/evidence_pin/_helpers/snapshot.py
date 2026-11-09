"""The pinned snapshot and clone that scripts/evidence.sh fetches."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from scripts.evidence_pin.constants import (
    CLONE_ENVIRONMENT,
    DEFAULT_CLONE_DIRECTORY,
    EVIDENCE_SCRIPT,
    EVIDENCE_SCRIPT_PREFIX,
    ROOT_ENVIRONMENT,
    UNPINNED_READ_ENVIRONMENT,
)
from scripts.evidence_pin.exceptions import SnapshotError


def evidence_clone(repository: Path) -> Path:
    """The evidence clone scripts/evidence.sh uses for the SQLQuack checkout `repository`."""
    return Path(os.environ.get(CLONE_ENVIRONMENT) or repository / DEFAULT_CLONE_DIRECTORY)


def pinned_snapshot(repository: Path) -> Path:
    """Clone the evidence if needed, then fetch, verify and return the pinned commit's snapshot."""
    environment: dict[str, str] = {**os.environ, ROOT_ENVIRONMENT: str(repository)}
    environment.pop(UNPINNED_READ_ENVIRONMENT, None)
    path: str = ""
    for command in ("clone", "path"):
        result: subprocess.CompletedProcess[str] = subprocess.run(
            ("bash", str(EVIDENCE_SCRIPT), command),
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            message: str = result.stderr.strip().removeprefix(EVIDENCE_SCRIPT_PREFIX)
            raise SnapshotError(
                message or f"{EVIDENCE_SCRIPT.name} {command} failed with exit {result.returncode}"
            )
        sys.stderr.write(result.stderr)
        path = result.stdout.strip()
    return Path(path)
