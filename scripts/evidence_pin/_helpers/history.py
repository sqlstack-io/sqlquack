"""Checking where the pinned commit sits in the evidence repository's history."""

from __future__ import annotations

from pathlib import Path, PurePosixPath

from scripts.evidence_pin._helpers.git import blob, changed_paths, fetch, has_commit, is_ancestor
from scripts.evidence_pin._helpers.pin_file import parsed_pin
from scripts.evidence_pin.constants import (
    ADDED_STATUS,
    APPEND_ONLY_FOLDERS,
    APPEND_ONLY_NAMES,
    DELETED_STATUS,
    EVIDENCE_MAIN_BRANCH,
    MODIFIED_STATUS,
    PIN_FILE,
    PLACEHOLDER_COMMIT,
    SHORT_COMMIT,
)
from scripts.evidence_pin.exceptions import PinFileError
from scripts.evidence_pin.models import ChangedPath, EvidencePin


def main_branch_problems(*, clone: Path, commit: str) -> list[str]:
    """Why `commit` is not reachable from the evidence repository's main, if it is not."""
    if is_ancestor(repository=clone, ancestor=commit, descendant=EVIDENCE_MAIN_BRANCH):
        return []
    _ = fetch(repository=clone)
    if is_ancestor(repository=clone, ancestor=commit, descendant=EVIDENCE_MAIN_BRANCH):
        return []
    return [f"commit {commit[:SHORT_COMMIT]} is not reachable from the evidence repository's main"]


def moved_pin_problems(*, repository: Path, clone: Path, base: str, pin: EvidencePin) -> list[str]:
    """Why the pin, if it moved since SQLQuack revision `base`, did not move forward append-only."""
    previous: bytes | None = blob(repository=repository, revision=base, path=PIN_FILE)
    if previous is None:
        return []
    try:
        old: str = parsed_pin(previous.decode("utf-8")).commit
    except (UnicodeDecodeError, PinFileError):
        return []
    if old in (PLACEHOLDER_COMMIT, pin.commit):
        return []
    if not has_commit(repository=clone, commit=old):
        _ = fetch(repository=clone)
    if not has_commit(repository=clone, commit=old):
        return [f"the previous pin {old[:SHORT_COMMIT]} is not in the evidence repository"]
    if not is_ancestor(repository=clone, ancestor=old, descendant=pin.commit):
        return [
            f"commit {pin.commit[:SHORT_COMMIT]} does not descend from {old[:SHORT_COMMIT]}, "
            "the previous pin: pins only move forward"
        ]
    problems: list[str] = []
    for change in changed_paths(repository=clone, old=old, new=pin.commit):
        if _append_only(change.path):
            problems.extend(_change_problems(clone=clone, old=old, new=pin.commit, change=change))
    return problems


def _append_only(path: str) -> bool:
    """Whether `path` holds recorded evidence, which may only grow."""
    parts: tuple[str, ...] = PurePosixPath(path).parts
    return parts[-1] in APPEND_ONLY_NAMES or not APPEND_ONLY_FOLDERS.isdisjoint(parts[:-1])


def _change_problems(*, clone: Path, old: str, new: str, change: ChangedPath) -> list[str]:
    """Why one change to recorded evidence between `old` and `new` is not an addition or append."""
    span: str = f"between {old[:SHORT_COMMIT]} and {new[:SHORT_COMMIT]}"
    if change.status == ADDED_STATUS:
        return []
    if change.status == DELETED_STATUS:
        return [f"{change.path}: recorded evidence is append-only, but it was deleted {span}"]
    if change.status != MODIFIED_STATUS:
        return [f"{change.path}: recorded evidence is append-only (status {change.status} {span})"]
    before: bytes = blob(repository=clone, revision=old, path=change.path) or b""
    after: bytes = blob(repository=clone, revision=new, path=change.path) or b""
    if after.startswith(before):
        return []
    return [
        f"{change.path}: recorded evidence is append-only, but it was rewritten {span}, "
        "not only appended to"
    ]
