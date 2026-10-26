"""Reading a compiled CREATE FUNCTION header."""

from __future__ import annotations

import re

from scripts.census_functions.constants import (
    CLOSE_PAREN,
    HEADER_PATTERN,
    LANGUAGE_PATTERN,
    OPEN_PAREN,
    SCALAR_PLACEHOLDER,
    SCALAR_TYPE,
    SEPARATOR,
    SQL_LANGUAGE,
    TABLE_PLACEHOLDER,
    TABLE_PREFIX,
)
from scripts.census_functions.models import Signature


def depth_change(character: str) -> int:
    """How a character changes the parenthesis depth."""
    return (character == OPEN_PAREN) - (character == CLOSE_PAREN)


def top_level_parts(text: str) -> list[str]:
    """`text` split at commas outside parentheses, each part stripped; empty text has none."""
    parts: list[str] = []
    depth: int = 0
    current: list[str] = []
    for character in text:
        depth += depth_change(character)
        if character == SEPARATOR and depth == 0:
            parts.append("".join(current).strip())
            current = []
            continue
        current.append(character)
    tail: str = "".join(current).strip()
    return [*parts, tail] if tail else parts


def balanced(text: str) -> tuple[str, str]:
    """The text before the parenthesis closing an already open one, and the text after it."""
    depth: int = 1
    for index, character in enumerate(text):
        depth += depth_change(character)
        if depth == 0:
            return text[:index], text[index + 1 :]
    return text, ""


def named_types(text: str) -> tuple[tuple[str, str], ...]:
    """`name TYPE, ...` as upper-case names and types."""
    pairs: list[tuple[str, str]] = []
    for part in top_level_parts(text):
        name, _, data_type = part.partition(" ")
        pairs.append((name.strip('"').upper(), data_type.strip().upper()))
    return tuple(pairs)


def signature(header: str) -> Signature | None:
    """The signature in a compiled function's text, or None when its header does not match."""
    found: re.Match[str] | None = HEADER_PATTERN.search(header)
    if found is None:
        return None
    returns_text: str = found.group("returns").strip()
    table: re.Match[str] | None = TABLE_PREFIX.match(returns_text)
    scalar: re.Match[str] | None = SCALAR_TYPE.match(returns_text)
    if table is not None:
        columns, rest = balanced(returns_text[table.end() :])
        returns: str | tuple[tuple[str, str], ...] = named_types(columns)
    elif scalar is not None:
        returns, rest = scalar.group(0).upper(), returns_text[scalar.end() :]
    else:
        return None
    language: re.Match[str] | None = LANGUAGE_PATTERN.search(rest)
    return Signature(
        language=language.group(1).lower() if language else SQL_LANGUAGE,
        arguments=named_types(found.group("arguments")),
        returns=returns,
    )


def placeholder(*, stem: str, table: bool) -> str:
    """The name the census gives `__udf("stem")` or `__table_fn("stem")` once resolved."""
    cleaned: str = re.sub(r"[^A-Za-z0-9]", "_", stem).upper()
    return f"{TABLE_PLACEHOLDER if table else SCALAR_PLACEHOLDER}{cleaned}"
