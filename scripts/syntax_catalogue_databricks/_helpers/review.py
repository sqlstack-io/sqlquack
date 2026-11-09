"""Compare the omission audit with its reviewed audit.toml and summarise what it checked."""

from __future__ import annotations

import hashlib
from pathlib import Path

from scripts.syntax_catalogue_databricks.constants import (
    HELPER_FILES,
    LEXER_FILE,
    PARSER_FILE,
    REVIEW_BLOCK,
    REVIEW_KEYWORD,
    REVIEW_PAGE,
    REVIEW_RULE,
    REVIEW_TOKEN,
    REVIEW_WORD,
    SPARK_DIR,
    SYNONYM_TOKENS,
    UNSEEN_TOKENS,
)
from scripts.syntax_catalogue_databricks.models import AuditGap, AuditResult
from scripts.syntax_catalogue_databricks.types import Review


def source_hashes(cache: Path) -> dict[str, str]:
    """The review keys: SHA-256 of the pinned grammar, lexer and parser helper files."""
    return {
        f"{Path(name).stem}_sha256": hashlib.sha256(
            (cache / SPARK_DIR / Path(name).name).read_bytes()
        ).hexdigest()
        for name in (PARSER_FILE, LEXER_FILE, *HELPER_FILES)
    }


def rows(*, review: Review, table: str) -> list[dict[str, object]]:
    """A review table's rows, or none."""
    found: object = review.get(table, [])
    return [row for row in found if isinstance(row, dict)] if isinstance(found, list) else []


def covers(*, row: dict[str, object], gap: AuditGap) -> bool:
    """True when a [[gap]] row names the gap's rule and alternative and lists or contains it."""
    if row.get(REVIEW_RULE) != gap.rule or row.get("alternative") != gap.alternative:
        return False
    listed: object = row.get("sequences")
    if isinstance(listed, list):
        return gap.keywords in listed
    words: tuple[str, ...] = tuple(str(row.get("contains", "")).split())
    found: tuple[str, ...] = tuple(gap.keywords.split())
    return any(found[i : i + len(words)] == words for i in range(len(found) - len(words) + 1))


def reviewed_gaps(review: Review) -> set[str]:
    """Every exact `rule: keywords` sequence the [[gap]] rows list; they stay root gaps."""
    found: set[str] = set()
    for row in rows(review=review, table="gap"):
        listed: object = row.get("sequences")
        if isinstance(listed, list):
            found |= {f"{row[REVIEW_RULE]}: {sequence}" for sequence in listed}
        elif row.get("contains"):
            found.add(f"{row[REVIEW_RULE]}: {row['contains']}")
    return found


def gap_problems(*, gaps: list[AuditGap], review: Review) -> list[str]:
    """Gaps no row covers, rows that cover no gap, and listed sequences that are no gap."""
    table: list[dict[str, object]] = rows(review=review, table="gap")
    used: set[int] = set()
    problems: list[str] = []
    for gap in gaps:
        owners: list[int] = [i for i, row in enumerate(table) if covers(row=row, gap=gap)]
        used.update(owners)
        if not owners:
            problems.append(f"unreviewed gap: {gap.rule} #{gap.alternative}: {gap.keywords}")
    problems += [
        f"gap review covers nothing current: {row[REVIEW_RULE]} #{row.get('alternative')}: "
        f"{row.get('contains', '')}"
        for i, row in enumerate(table)
        if i not in used
    ]
    current: set[tuple[object, object, str]] = {(g.rule, g.alternative, g.keywords) for g in gaps}
    for row in table:
        listed: object = row.get("sequences")
        if isinstance(listed, list):
            problems += [
                f"gap review lists a sequence that is no gap: {row[REVIEW_RULE]} "
                f"#{row.get('alternative')}: {sequence}"
                for sequence in listed
                if (row[REVIEW_RULE], row.get("alternative"), sequence) not in current
            ]
    return problems


def compare(*, kind: str, current: set[str], reviewed: set[str]) -> list[str]:
    """Unreviewed current items and reviews that name nothing current."""
    return [f"unreviewed {kind}: {item}" for item in sorted(current - reviewed)] + [
        f"{kind} review names nothing current: {item}" for item in sorted(reviewed - current)
    ]


def named(*, review: Review, table: str, key: str) -> set[str]:
    """The names a review table's rows give under one key."""
    return {str(row[key]) for row in rows(review=review, table=table)}


def problems_of(*, result: AuditResult, review: Review, hashes: dict[str, str]) -> list[str]:
    """Unreviewed gaps and names, stale reviews and hashes, and an unseen rule left unwalked."""
    problems: list[str] = [
        f"{key} is {review.get(key)!r}, the pinned source is {value}"
        for key, value in hashes.items()
        if review.get(key) != value
    ]
    problems += gap_problems(gaps=result.gaps, review=review)
    for kind, table, key, current in (
        (
            "keyword",
            "keyword",
            REVIEW_KEYWORD,
            set(result.missing_keywords + result.unused_keywords),
        ),
        ("helper word", "word", REVIEW_WORD, set(result.missing_words)),
        ("index page", "index", REVIEW_PAGE, set(result.missing_index)),
        ("notation block", "block", REVIEW_BLOCK, set(result.notation_blocks)),
        ("unwalked rule", "unwalked", REVIEW_RULE, set(result.unwalked)),
        ("unseen token", "unseen", REVIEW_TOKEN, set(UNSEEN_TOKENS)),
        ("synonym", "synonym", "token", set(SYNONYM_TOKENS)),
    ):
        problems += compare(
            kind=kind, current=current, reviewed=named(review=review, table=table, key=key)
        )
    problems += [
        f"unseen token the walked grammar does not use: {token}"
        for token in UNSEEN_TOKENS
        if token not in result.uncompared_tokens
    ]
    problems += [
        f"uncompared token without an [[unseen]] entry: {token}"
        for token in result.uncompared_tokens
        if token not in UNSEEN_TOKENS
    ]
    return problems


def audit_summary(*, result: AuditResult, review: Review, problems: list[str]) -> dict[str, object]:
    """The counts the audit prints."""
    return {
        "rules": len(result.walked),
        "rules_unwalked": len(result.unwalked),
        "productions": result.productions_checked,
        "keyword_sequences": result.sequences_checked,
        "gaps": len(result.gaps),
        "gaps_derived": result.derived_gaps,
        "gap_rows": len(rows(review=review, table="gap")),
        "unseen_reviewed": len(rows(review=review, table="unseen")),
        "keywords": result.keywords_checked,
        "keywords_not_catalogued": len(result.missing_keywords),
        "keywords_unused_by_grammar": len(result.unused_keywords),
        "helper_words": result.words_checked,
        "helper_words_not_catalogued": len(result.missing_words),
        "index_entries": result.index_checked,
        "index_entries_not_pinned": len(result.missing_index),
        "other_blocks": result.blocks_checked,
        "notation_blocks": len(result.notation_blocks),
        "problems": len(problems),
    }
