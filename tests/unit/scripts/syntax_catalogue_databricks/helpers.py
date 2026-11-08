"""The committed [[unseen]] rows and an audit result for the review's guard."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.syntax_catalogue_databricks._helpers.audit import AuditResult
from scripts.syntax_catalogue_databricks.constants import AUDIT_FILE, UNSEEN_TOKENS
from scripts.syntax_catalogue_databricks.types import Review

ROOT: Path = Path(__file__).parents[4] / "coverage" / "syntax-databricks"


def review_without(removed: frozenset[str]) -> Review:
    """The committed audit.toml's [[unseen]] and [[synonym]] rows, less unseen rows of `removed`."""
    review: dict[str, list[dict[str, object]]] = tomllib.loads((ROOT / AUDIT_FILE).read_text())
    return {
        "unseen": [row for row in review["unseen"] if str(row["token"]) not in removed],
        "synonym": review["synonym"],
    }


def result_with_tokens(
    *, unused: frozenset[str], extra: frozenset[str], blocks: tuple[str, ...] = ()
) -> AuditResult:
    """An audit result whose walked grammar uses every listed token but `unused`, and `extra`;
    `blocks` are the notation blocks it found."""
    return AuditResult(
        gaps=[],
        derived_gaps=0,
        missing_keywords=[],
        unused_keywords=[],
        missing_words=[],
        missing_index=[],
        notation_blocks=list(blocks),
        walked=[],
        unwalked=[],
        uncompared_tokens=sorted((set(UNSEEN_TOKENS) - unused) | extra),
        sequences_checked=0,
        productions_checked=0,
        keywords_checked=0,
        words_checked=0,
        index_checked=0,
        blocks_checked=len(blocks),
    )
