"""The committed [[unseen]] rows and an audit result for the review's guard."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.syntax_catalogue_databricks.constants import AUDIT_FILE, MANUAL_PREFIX, UNSEEN_TOKENS
from scripts.syntax_catalogue_databricks.models import AuditResult
from scripts.syntax_catalogue_databricks.types import Review

ROOT: Path = Path(__file__).parents[4] / "coverage" / "syntax-databricks"
URL: str = f"{MANUAL_PREFIX}sql-ref-example"
ARTICLE: str = "<article><h1>EXAMPLE</h1><p>Text.</p></article>"


def review_without(removed: frozenset[str]) -> Review:
    """The committed audit.toml's [[unseen]] and [[synonym]] rows, less unseen rows of `removed`."""
    review: dict[str, list[dict[str, object]]] = tomllib.loads((ROOT / AUDIT_FILE).read_text())
    return {
        "unseen": list(filter(lambda row: str(row["token"]) not in removed, review["unseen"])),
        "synonym": review["synonym"],
    }


def result_with_tokens(
    *, unused: frozenset[str], extra: frozenset[str], blocks: tuple[str, ...] = ()
) -> AuditResult:
    """An audit result using every listed token but `unused`, plus `extra`, with notation blocks."""
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


def page(*, bundle: str, article: str) -> str:
    """A reference page with one script bundle and one article."""
    return f"<html><script src=/assets/js/main.{bundle}.js></script>{article}</html>"


def cached(root: Path, html: str) -> Path:
    """A cache holding the one page."""
    (root / "pages").mkdir(parents=True)
    (root / "pages" / "sql-ref-example.html").write_text(html)
    return root
