"""Reading commits, changes and blobs from Git."""

from __future__ import annotations

import subprocess
from pathlib import Path

from scripts.evidence_pin.models import ChangedPath


def _git(*, repository: Path, arguments: tuple[str, ...]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        ("git", "-C", str(repository), *arguments), check=False, capture_output=True
    )


def has_commit(*, repository: Path, commit: str) -> bool:
    """Whether `repository` holds the commit `commit`."""
    arguments: tuple[str, ...] = ("cat-file", "-e", f"{commit}^{{commit}}")
    return _git(repository=repository, arguments=arguments).returncode == 0


def is_ancestor(*, repository: Path, ancestor: str, descendant: str) -> bool:
    """Whether `ancestor` is `descendant` or one of its ancestors in `repository`."""
    arguments: tuple[str, ...] = ("merge-base", "--is-ancestor", ancestor, descendant)
    return _git(repository=repository, arguments=arguments).returncode == 0


def fetch(*, repository: Path) -> bool:
    """Fetch every branch of the clone `repository` from origin; whether that worked."""
    return _git(repository=repository, arguments=("fetch", "--quiet", "origin")).returncode == 0


def blob(*, repository: Path, revision: str, path: str) -> bytes | None:
    """The bytes of `path` at `revision`, absent when it does not exist there."""
    result: subprocess.CompletedProcess[bytes] = _git(
        repository=repository, arguments=("show", f"{revision}:{path}")
    )
    return result.stdout if result.returncode == 0 else None


def changed_paths(*, repository: Path, old: str, new: str) -> list[ChangedPath]:
    """Every file that differs between the commits `old` and `new`, renames as delete and add."""
    output: bytes = subprocess.run(
        (
            "git",
            "-C",
            str(repository),
            "diff-tree",
            "-r",
            "-z",
            "--no-renames",
            "--name-status",
            old,
            new,
        ),
        check=True,
        capture_output=True,
    ).stdout
    fields: list[str] = output.decode("utf-8", errors="surrogateescape").split("\0")
    return [
        ChangedPath(status=status, path=path)
        for status, path in zip(fields[0:-1:2], fields[1::2], strict=False)
    ]
