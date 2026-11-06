"""Other spellings of one statement: case, whitespace, a final semicolon, comments, quoting."""

from __future__ import annotations

import random
import re
from collections.abc import Callable

from scripts.basics_corpus.constants import (
    BLOCK,
    BLOCK_COMMENT,
    CASINGS,
    CLAUSES,
    COMMENTS,
    ENDING_SHARE,
    ENDINGS,
    HALF,
    KEYWORDS,
    LEADING,
    LEADING_COMMENT,
    LOWER_CASE,
    MIXED_CASE,
    QUOTE,
    QUOTED_TABLE,
    QUOTED_TABLE_SHARE,
    SPACINGS,
    TRAILING,
    TRAILING_COMMENT,
    UNSPACED,
    UPPER_CASE,
)

_KEYWORD_PATTERN: re.Pattern[str] = re.compile(
    r"\b(" + "|".join(keyword.replace(" ", r"\s+") for keyword in KEYWORDS) + r")\b"
)
_TABLE_PATTERN: re.Pattern[str] = re.compile(r"\borders\b", re.IGNORECASE)


def respelled(*, sql: str, rng: random.Random) -> str:
    """`sql` spelled another way that means the same statement in Snowflake."""
    casing: str = rng.choice(CASINGS)
    out: str = _outside_strings(
        sql=sql, change=lambda text: _cased(text=text, casing=casing, rng=rng)
    )
    spacing: str = rng.choice([*SPACINGS, UNSPACED])
    if spacing in SPACINGS:
        out = _outside_strings(
            sql=out, change=lambda text: _spaced(text=text, space=SPACINGS[spacing])
        )
    if rng.random() < ENDING_SHARE:
        out += rng.choice(ENDINGS)
    comment: str | None = rng.choice(COMMENTS)
    if comment == LEADING:
        out = LEADING_COMMENT + out
    elif comment == TRAILING:
        out = out.rstrip(";\n ") + TRAILING_COMMENT
    elif comment == BLOCK:
        out = out.replace(" ", BLOCK_COMMENT, 1)
    if rng.random() < QUOTED_TABLE_SHARE:
        out = _outside_strings(sql=out, change=lambda text: _TABLE_PATTERN.sub(QUOTED_TABLE, text))
    return out


def _cased(*, text: str, casing: str, rng: random.Random) -> str:
    if casing == LOWER_CASE:
        return text.lower()
    if casing == UPPER_CASE:
        return text.upper()
    if casing == MIXED_CASE:
        return "".join(c.upper() if rng.random() < HALF else c.lower() for c in text)
    return _KEYWORD_PATTERN.sub(lambda match: match.group(0).lower(), text)


def _spaced(*, text: str, space: str) -> str:
    for clause in CLAUSES:
        pattern: re.Pattern[str] = re.compile(
            r" (" + clause.replace(" ", r"\s+") + r")\b", re.IGNORECASE
        )
        text = pattern.sub(lambda match: space + match.group(1), text)
    return text


def _outside_strings(*, sql: str, change: Callable[[str], str]) -> str:
    """`change` applied to the text outside single-quoted literals."""
    parts: list[str] = sql.split(QUOTE)
    return QUOTE.join(change(part) if index % 2 == 0 else part for index, part in enumerate(parts))
