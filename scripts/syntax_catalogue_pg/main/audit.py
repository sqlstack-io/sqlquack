"""Check the catalogue against gram.y and kwlist.h: every gap needs a reviewed reason."""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

from scripts.syntax_catalogue_pg._helpers.audit import audit
from scripts.syntax_catalogue_pg._helpers.fetch import pinned
from scripts.syntax_catalogue_pg._helpers.review import audit_summary, grammar_hashes, problems_of
from scripts.syntax_catalogue_pg.constants import AUDIT_FILE
from scripts.syntax_catalogue_pg.models import AuditResult


def run_audit(argv: list[str] | None = None) -> int:
    """Print the audit's counts; exit 1 when anything is unreviewed or stale."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--root", type=Path, default=Path("coverage/syntax-postgres"))
    arguments: argparse.Namespace = parser.parse_args(argv)
    tree, _ = pinned(arguments.cache)
    result: AuditResult = audit(tree=tree, root=arguments.root)
    review: dict[str, object] = tomllib.loads((arguments.root / AUDIT_FILE).read_text())
    problems: list[str] = problems_of(result=result, review=review, hashes=grammar_hashes(tree))
    print(json.dumps(audit_summary(result=result, problems=problems)))
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    return 0
