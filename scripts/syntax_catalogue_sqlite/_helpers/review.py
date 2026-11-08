"""Compare the omission audit with its reviewed audit.toml and summarise what it checked."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from scripts.syntax_catalogue_sqlite._helpers.grammar import source_text
from scripts.syntax_catalogue_sqlite.constants import (
    ENGINE_EXTRACT,
    GRAMMAR_FILE,
    KEYWORD_FILE,
    REVIEW_KEYWORD,
    REVIEW_NAME,
    REVIEW_RULE,
    UNSEEN_RULES,
)
from scripts.syntax_catalogue_sqlite.models import AuditResult
from scripts.syntax_catalogue_sqlite.types import Review


def source_hashes(cache: Path) -> dict[str, str]:
    """The review keys: SHA-256 of parse.y, of mkkeywordhash.c and of the engine extract."""
    extract: object = json.loads((cache / ENGINE_EXTRACT).read_text())
    return {
        "parse_y_sha256": hashlib.sha256(
            source_text(cache=cache, name=GRAMMAR_FILE).encode()
        ).hexdigest(),
        "mkkeywordhash_sha256": hashlib.sha256(
            source_text(cache=cache, name=KEYWORD_FILE).encode()
        ).hexdigest(),
        "engine_extract_sha256": hashlib.sha256(
            json.dumps(extract, sort_keys=True).encode()
        ).hexdigest(),
    }


def rows(*, review: Review, table: str) -> list[dict[str, object]]:
    """A review table's rows, or none."""
    found: object = review.get(table, [])
    return [row for row in found if isinstance(row, dict)] if isinstance(found, list) else []


def reviewed_gaps(review: Review) -> set[str]:
    """Every `rule: keywords` the [[gap]] rows review."""
    found: set[str] = set()
    for row in rows(review=review, table="gap"):
        sequences: object = row.get("sequences", [])
        if isinstance(sequences, list):
            found |= {f"{row[REVIEW_RULE]}: {sequence}" for sequence in sequences}
    return found


def compare(*, kind: str, current: set[str], reviewed: set[str]) -> list[str]:
    """Unreviewed current items and reviews that name nothing current."""
    return [f"unreviewed {kind}: {item}" for item in sorted(current - reviewed)] + [
        f"{kind} review names nothing current: {item}" for item in sorted(reviewed - current)
    ]


def named(*, review: Review, table: str, key: str) -> set[str]:
    """The names a review table's rows give under one key."""
    return {str(row[key]) for row in rows(review=review, table=table)}


def problems_of(*, result: AuditResult, review: Review, hashes: dict[str, str]) -> list[str]:
    """Unreviewed gaps and names, stale reviews and hashes, and a parser that differs."""
    problems: list[str] = [
        f"{key} is {review.get(key)!r}, the pinned source is {value}"
        for key, value in hashes.items()
        if review.get(key) != value
    ]
    if result.productions_read != result.productions_generated:
        problems.append(
            f"parse.y read as {result.productions_read} productions; the amalgamation's parser "
            f"has {result.productions_generated}"
        )
    problems += compare(
        kind="gap",
        current={f"{g.rule}: {g.keywords}" for g in result.gaps},
        reviewed=reviewed_gaps(review),
    )
    problems += compare(
        kind="keyword",
        current={k.name for k in result.missing_keywords + result.unused_keywords}
        | set(result.engine_keywords),
        reviewed=named(review=review, table="keyword", key=REVIEW_KEYWORD),
    )
    for table, missing in (
        ("function", result.missing_functions),
        ("pragma", result.missing_pragmas),
        ("type", result.missing_types),
    ):
        problems += compare(
            kind=table,
            current={n.name for n in missing},
            reviewed=named(review=review, table=table, key=REVIEW_NAME),
        )
    problems += compare(
        kind="unseen rule",
        current=set(UNSEEN_RULES),
        reviewed=named(review=review, table="unseen", key=REVIEW_RULE),
    )
    problems += [
        f"unseen rule the walk does not reach: {rule}"
        for rule in UNSEEN_RULES
        if rule not in result.walked
    ]
    problems += compare(
        kind="unwalked rule",
        current=set(result.unwalked),
        reviewed=named(review=review, table="unwalked", key=REVIEW_RULE),
    )
    return problems


def audit_summary(*, result: AuditResult, review: Review, problems: list[str]) -> dict[str, object]:
    """The counts the audit prints."""
    return {
        "rules": result.rules_checked,
        "rules_unwalked": len(result.unwalked),
        "productions": result.productions_checked,
        "productions_in_parser": result.productions_generated,
        "unseen_reviewed": len(rows(review=review, table="unseen")),
        "keyword_sequences": result.sequences_checked,
        "gaps_reviewed": len(result.gaps),
        "gaps_derived": result.derived_gaps,
        "keywords": result.keywords_checked,
        "keywords_not_catalogued": len(result.missing_keywords),
        "keywords_unused_by_grammar": len(result.unused_keywords),
        "keywords_not_in_engine": len(result.engine_keywords),
        "functions": result.functions_checked,
        "functions_not_catalogued": len(result.missing_functions),
        "pragmas": result.pragmas_checked,
        "pragmas_not_catalogued": len(result.missing_pragmas),
        "types_and_collations": result.types_checked,
        "types_and_collations_not_catalogued": len(result.missing_types),
        "problems": len(problems),
    }
