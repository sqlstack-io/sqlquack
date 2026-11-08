"""The GoogleSQL keyword table of the pinned keywords.cc: each keyword and its class."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.syntax_catalogue_bigquery.constants import (
    GRAMMAR_DIR,
    KEYWORD_CLASSES,
    KEYWORD_ENTRY,
    KEYWORDS_FILE,
    KEYWORDS_TABLE_END,
    KEYWORDS_TABLE_START,
    SPANNER_KEYWORDS_END,
)
from scripts.syntax_catalogue_bigquery.exceptions import BigQueryCatalogueError


def keyword_table(cache: Path) -> dict[str, str]:
    """Upper-cased keyword -> class (reserved, nonreserved, conditional, spanner)."""
    text: str = (cache / GRAMMAR_DIR / KEYWORDS_FILE).read_text()
    start: int = text.find(KEYWORDS_TABLE_START)
    end: int = text.find(KEYWORDS_TABLE_END, start)
    if start < 0 or end < 0:
        raise BigQueryCatalogueError(f"{KEYWORDS_FILE}: kAllKeywords table not found")
    table: str = text[start:end]
    spanner_end: int = table.find(SPANNER_KEYWORDS_END)
    found: dict[str, str] = {}
    for match in KEYWORD_ENTRY.finditer(table):
        word: str = match.group(1).upper()
        if spanner_end >= 0 and match.start() < spanner_end:
            found[word] = "spanner"
            continue
        declared: str = match.group(2) or ""
        found[word] = KEYWORD_CLASSES.get(declared, "nonreserved")
    if not found:
        raise BigQueryCatalogueError(f"{KEYWORDS_FILE}: no keyword entries read")
    return found


def keyword_words(cache: Path) -> frozenset[str]:
    """Every keyword of the table, upper case."""
    return frozenset(keyword_table(cache))


def entry_count(cache: Path) -> int:
    """How many `{"word", ...}` entries kAllKeywords has, for the audit's own cross-check."""
    text: str = (cache / GRAMMAR_DIR / KEYWORDS_FILE).read_text()
    start: int = text.find(KEYWORDS_TABLE_START)
    table: str = text[start : text.find(KEYWORDS_TABLE_END, start)]
    return len(re.findall(r'\{"[a-z_]+",', table))
