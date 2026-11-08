"""Check the catalogue against the Spark grammar, its keywords and helpers: gaps need reasons."""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

from scripts.syntax_catalogue_databricks._helpers.audit import AuditResult, audit
from scripts.syntax_catalogue_databricks._helpers.fetch import pinned
from scripts.syntax_catalogue_databricks._helpers.review import (
    audit_summary,
    problems_of,
    reviewed_gaps,
    source_hashes,
)
from scripts.syntax_catalogue_databricks.constants import AUDIT_FILE


def run_audit(argv: list[str] | None = None) -> int:
    """Print the audit's counts; exit 1 when anything is unreviewed or stale."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--root", type=Path, default=Path("coverage/syntax-databricks"))
    arguments: argparse.Namespace = parser.parse_args(argv)
    record, _ = pinned(arguments.cache)
    review: dict[str, object] = tomllib.loads((arguments.root / AUDIT_FILE).read_text())
    result: AuditResult = audit(
        cache=arguments.cache,
        root=arguments.root,
        record=record,
        own=frozenset(reviewed_gaps(review)),
    )
    problems: list[str] = problems_of(
        result=result, review=review, hashes=source_hashes(arguments.cache)
    )
    print(json.dumps(audit_summary(result=result, review=review, problems=problems)))
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    return 0
