"""Evidence pin check entry point."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from scripts.evidence_pin._helpers.content_hash import content_sha256
from scripts.evidence_pin._helpers.history import main_branch_problems, moved_pin_problems
from scripts.evidence_pin._helpers.pin_file import parsed_pin
from scripts.evidence_pin._helpers.snapshot import evidence_clone, pinned_snapshot
from scripts.evidence_pin.constants import PIN_FILE, PLACEHOLDER_COMMIT, SHORT_COMMIT
from scripts.evidence_pin.exceptions import PinFileError, SnapshotError
from scripts.evidence_pin.models import EvidencePin


def run_pin_check(argv: list[str] | None = None) -> int:
    """Check evidence.toml's shape and snapshot hash, its commit's place on main, and its moves."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=run_pin_check.__doc__)
    parser.add_argument("--base", help="the SQLQuack revision whose pin the current one replaces")
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    arguments: argparse.Namespace = parser.parse_args(argv)
    repository: Path = arguments.repository.resolve()
    try:
        pin: EvidencePin = parsed_pin((repository / PIN_FILE).read_text(encoding="utf-8"))
        if pin.commit == PLACEHOLDER_COMMIT:
            print("evidence pin: no evidence pinned yet", file=sys.stderr)
            return 0
        snapshot: Path = pinned_snapshot(repository)
    except (OSError, PinFileError, SnapshotError) as error:
        return _failed([str(error)])
    actual: str = content_sha256(snapshot)
    if actual != pin.content_sha256:
        return _failed(
            [
                f"content hash mismatch in {snapshot}: {PIN_FILE} expects {pin.content_sha256}, "
                f"but its files hash to {actual}"
            ]
        )
    clone: Path = evidence_clone(repository)
    problems: list[str] = main_branch_problems(clone=clone, commit=pin.commit)
    if arguments.base is not None:
        problems.extend(
            moved_pin_problems(repository=repository, clone=clone, base=arguments.base, pin=pin)
        )
    if problems:
        return _failed(problems)
    print(f"evidence pin: passed, commit {pin.commit[:SHORT_COMMIT]}", file=sys.stderr)
    return 0


def _failed(problems: list[str]) -> int:
    for problem in problems:
        print(f"evidence pin: {problem}", file=sys.stderr)
    return 1
