"""The committed audit.toml and an audit result that every committed row reviews."""

from __future__ import annotations

import itertools
import tomllib
from pathlib import Path
from typing import cast

from scripts.syntax_catalogue_sqlite.constants import AUDIT_FILE, UNSEEN_RULES
from scripts.syntax_catalogue_sqlite.models import AuditGap, AuditResult, EngineName
from scripts.syntax_catalogue_sqlite.types import Review

ROOT: Path = Path(__file__).parents[4] / "coverage" / "syntax-sqlite"
GENERATED_RULES: int = 412


def committed_review() -> Review:
    """The committed audit.toml."""
    return tomllib.loads((ROOT / AUDIT_FILE).read_text())


def review_without(*, table: str, key: str, removed: str) -> Review:
    """The committed review less every row of one table whose `key` is `removed`."""
    review: Review = committed_review()
    rows: list[dict[str, object]] = cast("list[dict[str, object]]", review[table])
    by_key: dict[str, list[dict[str, object]]] = {}
    for row in rows:
        by_key.setdefault(str(row[key]), []).append(row)
    remaining: list[str] = sorted(by_key.keys() - {removed})
    review[table] = list(itertools.chain.from_iterable(by_key[name] for name in remaining))
    return review


def gaps_of(row: dict[str, object]) -> list[AuditGap]:
    """The gaps one [[gap]] row reviews."""
    sequences: list[str] = cast("list[str]", row["sequences"])
    return [AuditGap(rule=str(row["rule"]), alternative=1, keywords=s) for s in sequences]


def reviewed_result(
    *, unwalked: frozenset[str] = frozenset(), productions_read: int = GENERATED_RULES
) -> AuditResult:
    """An audit result whose every finding a committed row reviews, as the pinned audit finds."""
    review: Review = committed_review()
    rows: list[dict[str, object]] = cast("list[dict[str, object]]", review["gap"])
    gaps: list[AuditGap] = list(itertools.chain.from_iterable(map(gaps_of, rows)))
    walked: list[str] = sorted(set(UNSEEN_RULES) - unwalked)
    keywords: list[dict[str, object]] = cast("list[dict[str, object]]", review["keyword"])
    return AuditResult(
        gaps=gaps,
        derived_gaps=0,
        missing_keywords=[],
        unused_keywords=[EngineName(name=str(row["keyword"]), kind="") for row in keywords],
        engine_keywords=[str(row["keyword"]) for row in keywords],
        missing_functions=[
            EngineName(name=str(row["name"]), kind="")
            for row in cast("list[dict[str, object]]", review["function"])
        ],
        missing_pragmas=[],
        missing_types=[],
        sequences_checked=0,
        rules_checked=len(walked),
        productions_checked=productions_read,
        productions_read=productions_read,
        productions_generated=GENERATED_RULES,
        walked=walked,
        unwalked=[],
        functions_checked=0,
        keywords_checked=0,
        pragmas_checked=0,
        types_checked=0,
    )
