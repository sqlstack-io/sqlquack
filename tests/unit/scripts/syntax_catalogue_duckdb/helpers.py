"""The committed [[unseen]] rows and a walk result for the review's guard."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.syntax_catalogue_duckdb.constants import AUDIT_FILE, UNSEEN_RULES
from scripts.syntax_catalogue_duckdb.models import AuditResult
from scripts.syntax_catalogue_duckdb.types import Review

ROOT: Path = Path(__file__).parents[4] / "coverage" / "syntax-duckdb"


def review_without(removed: frozenset[str]) -> Review:
    """The committed audit.toml's [[unseen]] rows, less those naming a removed rule."""
    review: dict[str, list[dict[str, object]]] = tomllib.loads((ROOT / AUDIT_FILE).read_text())
    by_rule: dict[str, dict[str, object]] = {str(row["rule"]): row for row in review["unseen"]}
    return {"unseen": [by_rule[rule] for rule in sorted(by_rule.keys() - removed)]}


def result_walking_all_but(unwalked: frozenset[str]) -> AuditResult:
    """An audit result whose walk reaches every required [[unseen]] rule except `unwalked`."""
    walked: list[str] = sorted(set(UNSEEN_RULES) - unwalked)
    return AuditResult(
        gaps=[],
        derived_gaps=0,
        missing_keywords=[],
        unused_keywords=[],
        missing_functions=[],
        missing_types=[],
        sequences_checked=0,
        rules_checked=len(walked),
        walked=walked,
        unwalked=[],
        functions_checked=0,
        keywords_checked=0,
        types_checked=0,
    )
