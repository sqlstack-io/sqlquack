"""Check the catalogue against gram.y and kwlist.h: every gap needs a reviewed reason."""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

from scripts.syntax_catalogue._helpers.grammar import sha256
from scripts.syntax_catalogue_pg._helpers.audit import audit
from scripts.syntax_catalogue_pg._helpers.fetch import pinned
from scripts.syntax_catalogue_pg.constants import GRAM_Y, KWLIST_H
from scripts.syntax_catalogue_pg.models import AuditResult

AUDIT_FILE: str = "audit.toml"


def problems_of(
    *, result: AuditResult, review: dict[str, object], hashes: dict[str, str]
) -> list[str]:
    """Unreviewed gaps, reviews naming nothing current, and stale grammar hashes."""
    problems: list[str] = [
        f"{key} is {review.get(key)!r}, the pinned file is {value}"
        for key, value in hashes.items()
        if review.get(key) != value
    ]
    rows: object = review.get("production", [])
    reviewed: set[tuple[str, str]] = (
        {(str(r["rule"]), str(r["production"])) for r in rows} if isinstance(rows, list) else set()
    )
    current: set[tuple[str, str]] = {(g.rule, g.production) for g in result.gaps}
    problems += [f"unreviewed: {rule}: {prod}" for rule, prod in sorted(current - reviewed)]
    problems += [
        f"review names no gap: {rule}: {prod}" for rule, prod in sorted(reviewed - current)
    ]
    keyword_rows: object = review.get("keyword", [])
    keywords: set[str] = (
        {str(r["keyword"]) for r in keyword_rows} if isinstance(keyword_rows, list) else set()
    )
    missing: set[str] = {k.text for k in result.missing}
    problems += [f"unreviewed keyword: {k}" for k in sorted(missing - keywords)]
    problems += [
        f"keyword review names a catalogued keyword: {k}" for k in sorted(keywords - missing)
    ]
    return problems


def run_audit(argv: list[str] | None = None) -> int:
    """Print the audit's counts; exit 1 when anything is unreviewed or stale."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--root", type=Path, default=Path("coverage/syntax-postgres"))
    arguments: argparse.Namespace = parser.parse_args(argv)
    tree, _ = pinned(arguments.cache)
    result: AuditResult = audit(tree=tree, root=arguments.root)
    review: dict[str, object] = tomllib.loads((arguments.root / AUDIT_FILE).read_text())
    hashes: dict[str, str] = {
        "gram_y_sha256": sha256((tree / GRAM_Y).read_text()),
        "kwlist_h_sha256": sha256((tree / KWLIST_H).read_text()),
    }
    problems: list[str] = problems_of(result=result, review=review, hashes=hashes)
    print(
        json.dumps(
            {
                "rules": result.rules_checked,
                "keyword_sequences": result.alternatives_checked,
                "gap_productions": len({(g.rule, g.production) for g in result.gaps}),
                "keywords": result.keywords_checked,
                "keywords_unused_by_grammar": [k.text for k in result.unused],
                "keywords_not_catalogued": len(result.missing),
                "problems": len(problems),
            }
        )
    )
    if problems:
        print("\n".join(problems), file=sys.stderr)
        return 1
    return 0
