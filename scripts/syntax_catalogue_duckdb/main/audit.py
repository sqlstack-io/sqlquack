"""Check the catalogue against the PEG grammar and the engine: every gap needs a reviewed reason."""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

from scripts.syntax_catalogue_duckdb._helpers.audit import audit
from scripts.syntax_catalogue_duckdb._helpers.fetch import pinned
from scripts.syntax_catalogue_duckdb._helpers.pages import undecided_pages
from scripts.syntax_catalogue_duckdb._helpers.review import (
    audit_summary,
    problems_of,
    reviewed_gaps,
    source_hashes,
)
from scripts.syntax_catalogue_duckdb.constants import AUDIT_FILE, ENGINE_DIR, GRAMMAR_STATEMENTS
from scripts.syntax_catalogue_duckdb.models import AuditResult


def run_audit(argv: list[str] | None = None) -> int:
    """Print the audit's counts; exit 1 when anything is unreviewed or stale."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--root", type=Path, default=Path("coverage/syntax-duckdb"))
    arguments: argparse.Namespace = parser.parse_args(argv)
    cache, _ = pinned(arguments.cache)
    grammar: Path = cache / ENGINE_DIR / GRAMMAR_STATEMENTS
    review: dict[str, object] = tomllib.loads((arguments.root / AUDIT_FILE).read_text())
    result: AuditResult = audit(
        cache=cache, grammar=grammar, root=arguments.root, own=frozenset(reviewed_gaps(review))
    )
    hashes: dict[str, str] = source_hashes(cache=cache, grammar=grammar)
    problems: list[str] = undecided_pages(cache) + problems_of(
        result=result, review=review, hashes=hashes
    )
    print(json.dumps(audit_summary(result=result, review=review, problems=problems)))
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    return 0
