"""Check the catalogue against the GoogleSQL grammar: every gap needs a reviewed reason."""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

from scripts.syntax_catalogue_bigquery._helpers.audit import audit
from scripts.syntax_catalogue_bigquery._helpers.fetch import pinned
from scripts.syntax_catalogue_bigquery._helpers.review import (
    audit_summary,
    problems_of,
    reviewed_gaps,
    source_hashes,
)
from scripts.syntax_catalogue_bigquery.constants import AUDIT_FILE, DEFAULT_ROOT
from scripts.syntax_catalogue_bigquery.models import AuditResult


def run_audit(argv: list[str] | None = None) -> int:
    """Print the audit's counts; exit 1 when anything is unreviewed or stale."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--root", type=Path, default=Path(DEFAULT_ROOT))
    arguments: argparse.Namespace = parser.parse_args(argv)
    cache: Path = pinned(arguments.cache)
    review: dict[str, object] = tomllib.loads((arguments.root / AUDIT_FILE).read_text())
    result: AuditResult = audit(
        cache=cache, root=arguments.root, own=frozenset(reviewed_gaps(review))
    )
    problems: list[str] = problems_of(result=result, review=review, hashes=source_hashes(cache))
    print(json.dumps(audit_summary(result=result, review=review, problems=problems)))
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    return 0
