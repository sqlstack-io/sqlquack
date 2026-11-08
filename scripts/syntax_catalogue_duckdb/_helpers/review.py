"""Compare the omission audit with its reviewed audit.toml and summarise what it checked."""

from __future__ import annotations

import json
from pathlib import Path
from typing import cast

from scripts.syntax_catalogue._helpers.grammar import sha256
from scripts.syntax_catalogue_duckdb.constants import (
    ENGINE_EXTRACT,
    REVIEW_NAME,
    REVIEW_PREFIX,
    UNSEEN_RULES,
)
from scripts.syntax_catalogue_duckdb.models import AuditResult
from scripts.syntax_catalogue_duckdb.types import Review


def source_hashes(*, cache: Path, grammar: Path) -> dict[str, str]:
    """The review keys of the pinned grammar files and engine extract hashes."""
    files: list[Path] = sorted(grammar.glob("*.gram"))
    joined: str = "".join(f"{path.name}\n{path.read_text(encoding='utf-8')}" for path in files)
    extract: object = json.loads((cache / ENGINE_EXTRACT).read_text())
    return {
        "grammar_sha256": sha256(joined),
        "engine_extract_sha256": sha256(json.dumps(extract, sort_keys=True)),
    }


def rows(*, review: Review, table: str) -> list[dict[str, object]]:
    """A review table's rows, or none."""
    found: object = review.get(table, [])
    return [row for row in found if isinstance(row, dict)] if isinstance(found, list) else []


def reviewed_gaps(review: Review) -> set[str]:
    """Every `rule: keywords` the [[gap]] rows review."""
    found: set[str] = set()
    for r in rows(review=review, table="gap"):
        sequences: list[str] = cast("list[str]", r.get("sequences", []))
        found |= {f"{r['rule']}: {sequence}" for sequence in sequences}
    return found


def compare(*, kind: str, current: set[str], reviewed: set[str]) -> list[str]:
    """Unreviewed current items and reviews that name nothing current."""
    return [f"unreviewed {kind}: {item}" for item in sorted(current - reviewed)] + [
        f"{kind} review names nothing current: {item}" for item in sorted(reviewed - current)
    ]


def function_problems(*, result: AuditResult, review: Review) -> list[str]:
    """Functions with no form and no reviewed reason, by exact name or listed prefix."""
    functions: list[dict[str, object]] = rows(review=review, table="function")
    prefixes: list[str] = [str(r[REVIEW_PREFIX]) for r in functions if REVIEW_PREFIX in r]
    named: set[str] = {str(r[REVIEW_NAME]) for r in functions if REVIEW_NAME in r}
    missing: set[str] = {f.name for f in result.missing_functions}
    reviewed_prefixes: tuple[str, ...] = tuple(prefixes)
    unprefixed: set[str] = {n for n in missing if not n.startswith(reviewed_prefixes)}
    problems: list[str] = compare(kind="function", current=unprefixed, reviewed=named)
    for p in prefixes:
        if not any(n.startswith(p) for n in missing):
            problems.append(f"function prefix review matches nothing: {p}")
    return problems


def problems_of(*, result: AuditResult, review: Review, hashes: dict[str, str]) -> list[str]:
    """Unreviewed gaps, keywords, functions and types, stale reviews and stale source hashes."""
    problems: list[str] = [
        f"{key} is {review.get(key)!r}, the pinned source is {value}"
        for key, value in hashes.items()
        if review.get(key) != value
    ]
    problems += compare(
        kind="gap",
        current={f"{g.rule}: {g.keywords}" for g in result.gaps},
        reviewed=reviewed_gaps(review),
    )
    problems += compare(
        kind="keyword",
        current={k.name for k in result.missing_keywords},
        reviewed={str(r["keyword"]) for r in rows(review=review, table="keyword")},
    )
    problems += function_problems(result=result, review=review)
    problems += compare(
        kind="type",
        current={t.name for t in result.missing_types},
        reviewed={str(r["name"]) for r in rows(review=review, table="type")},
    )
    problems += compare(
        kind="unseen rule",
        current=set(UNSEEN_RULES),
        reviewed={str(r["rule"]) for r in rows(review=review, table="unseen")},
    )
    problems += [
        f"unseen rule the walk does not reach: {rule}"
        for rule in UNSEEN_RULES
        if rule not in result.walked
    ]
    problems += compare(
        kind="unwalked rule",
        current=set(result.unwalked),
        reviewed={str(r["rule"]) for r in rows(review=review, table="unwalked")},
    )
    return problems


def audit_summary(*, result: AuditResult, review: Review, problems: list[str]) -> dict[str, object]:
    """The counts the audit prints."""
    return {
        "rules": result.rules_checked,
        "rules_unwalked": len(result.unwalked),
        "unseen_reviewed": len(rows(review=review, table="unseen")),
        "keyword_sequences": result.sequences_checked,
        "gaps_reviewed": len(result.gaps),
        "gaps_derived": result.derived_gaps,
        "keywords": result.keywords_checked,
        "keywords_not_catalogued": len(result.missing_keywords),
        "keywords_unused_by_grammar": len(result.unused_keywords),
        "functions": result.functions_checked,
        "functions_not_catalogued": len(result.missing_functions),
        "types": result.types_checked,
        "types_not_catalogued": len(result.missing_types),
        "problems": len(problems),
    }
