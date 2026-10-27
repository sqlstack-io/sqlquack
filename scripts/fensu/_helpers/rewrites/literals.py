"""Edits naming the string literals that `==` and `!=` comparisons test against."""

from __future__ import annotations

import re

from scripts.fensu.constants import (
    BANG,
    COMPARED_LITERAL_NAMES,
    DOUBLE_QUOTE,
    EQUALS,
    NAME_SEPARATOR,
    STRING,
    TRIVIA_KINDS,
)
from scripts.fensu.exceptions import TransformError
from scripts.fensu.models import Edit, Token


def compared_literals(*, tokens: list[Token]) -> list[Token]:
    """The plain string literals that stand on either side of `==` or `!=`."""
    significant: list[Token] = [token for token in tokens if token.kind not in TRIVIA_KINDS]
    compared: list[Token] = []
    for position, token in enumerate(significant):
        if token.kind != STRING or not token.text.startswith(DOUBLE_QUOTE):
            continue
        before: list[str] = [other.text for other in significant[max(position - 2, 0) : position]]
        after: list[str] = [other.text for other in significant[position + 1 : position + 3]]
        if _is_comparison(before) or _is_comparison(after):
            compared.append(token)
    return compared


def constant_name(literal: str) -> str:
    """The constant naming a compared literal: an override, or its text upper-cased."""
    value: str = literal[1:-1]
    if value in COMPARED_LITERAL_NAMES:
        return COMPARED_LITERAL_NAMES[value]
    name: str = re.sub(r"[^A-Z0-9]+", NAME_SEPARATOR, value.upper()).strip(NAME_SEPARATOR)
    if not name or not name[0].isalpha() or value != value.upper():
        raise TransformError(f"name the compared literal {literal} in COMPARED_LITERAL_NAMES")
    return name


def literal_edits(*, literals: list[Token]) -> list[Edit]:
    """Edits replacing each compared literal with its constant's name."""
    return [
        Edit(start=token.start, end=token.end, replacement=constant_name(token.text))
        for token in literals
    ]


def _is_comparison(texts: list[str]) -> bool:
    return texts in ([EQUALS, EQUALS], [BANG, EQUALS])
