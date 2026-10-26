"""Polyglot guard entry point."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.polyglot_guard._helpers.metadata import cargo_metadata, dependents_of
from scripts.polyglot_guard.constants import ALLOWED_DEPENDENTS, POLYGLOT_PACKAGE
from scripts.polyglot_guard.types import Metadata


def run_polyglot_guard(argv: list[str] | None = None) -> int:
    """Fail when a workspace crate outside the allowed set reaches polyglot at runtime."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument(
        "--metadata", type=Path, help="cargo metadata JSON instead of running cargo"
    )
    arguments: argparse.Namespace = parser.parse_args(argv)
    metadata: Metadata = (
        json.loads(arguments.metadata.read_text())
        if arguments.metadata
        else cargo_metadata(repository=arguments.repository)
    )
    dependents: set[str] = dependents_of(metadata=metadata, package=POLYGLOT_PACKAGE)
    unexpected: list[str] = sorted(dependents - ALLOWED_DEPENDENTS)
    for crate in unexpected:
        print(f"polyglot guard: {crate} depends on {POLYGLOT_PACKAGE} at runtime", file=sys.stderr)
    print(
        f"polyglot guard: {'FAILED' if unexpected else 'passed'}, "
        f"{len(dependents)} crates reach {POLYGLOT_PACKAGE}: {', '.join(sorted(dependents))}",
        file=sys.stderr,
    )
    return 1 if unexpected else 0
