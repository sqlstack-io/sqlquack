"""Compare the omission audit with its reviewed audit.toml and summarise what it checked."""

from __future__ import annotations

from pathlib import Path

from scripts.syntax_catalogue._helpers.grammar import sha256
from scripts.syntax_catalogue_pg.constants import GRAM_Y, KWLIST_H
from scripts.syntax_catalogue_pg.models import AuditResult


def grammar_hashes(tree: Path) -> dict[str, str]:
    """The review keys of the pinned gram.y and kwlist.h hashes."""
    return {
        "gram_y_sha256": sha256((tree / GRAM_Y).read_text()),
        "kwlist_h_sha256": sha256((tree / KWLIST_H).read_text()),
    }


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
    unwalked_rows: object = review.get("unwalked", [])
    listed: set[str] = (
        {str(r["rule"]) for r in unwalked_rows} if isinstance(unwalked_rows, list) else set()
    )
    problems += [f"unlisted unwalked rule: {r}" for r in sorted(set(result.unwalked) - listed)]
    problems += [f"listed rule is walked: {r}" for r in sorted(listed - set(result.unwalked))]
    return problems


def audit_summary(*, result: AuditResult, problems: list[str]) -> dict[str, object]:
    """The counts the audit prints."""
    return {
        "rules": result.rules_checked,
        "rules_unwalked": len(result.unwalked),
        "keyword_sequences": result.alternatives_checked,
        "gap_productions": len({(g.rule, g.production) for g in result.gaps}),
        "keywords": result.keywords_checked,
        "keywords_unused_by_grammar": [k.text for k in result.unused],
        "keywords_not_catalogued": len(result.missing),
        "problems": len(problems),
    }
