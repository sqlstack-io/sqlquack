"""Edits turning `unreachable!` in `Result`-returning functions into returned internal errors."""

from __future__ import annotations

import re

from scripts.fensu.constants import (
    ARGUMENT_SEPARATOR,
    BANG,
    CLOSING_BRACKETS,
    FN,
    IDENT,
    INTERNAL_ERROR,
    OPEN_BRACE,
    OPEN_PAREN,
    OPENING_BRACKETS,
    PUNCT,
    RESULT_RETURN,
    SEMICOLON,
    TAIL_CLOSERS,
    TRIVIA_KINDS,
    UNREACHABLE,
    UNREACHABLE_MESSAGE,
)
from scripts.fensu.models import Edit, Token


def unreachable_edits(*, text: str, tokens: list[Token]) -> list[Edit]:
    """Edits rewriting each `unreachable!(...)` whose innermost function returns a `Result`."""
    significant: list[Token] = [token for token in tokens if token.kind not in TRIVIA_KINDS]
    bodies: list[tuple[int, int, bool]] = function_bodies(text=text, significant=significant)
    edits: list[Edit] = []
    for position, token in enumerate(significant[:-2]):
        if not _is_unreachable(significant=significant, position=position):
            continue
        enclosing: list[tuple[int, int, bool]] = [
            body for body in bodies if body[0] < token.start < body[1]
        ]
        if not enclosing:
            continue
        _, body_end, returns_result = max(enclosing, key=lambda body: body[0])
        if not returns_result:
            continue
        close: int = matching(significant=significant, at=position + 2)
        arguments: str = text[significant[position + 2].end : significant[close].start].strip()
        tail: bool = all(
            later.text in TAIL_CLOSERS
            for later in significant[close + 1 :]
            if later.start < body_end - 1
        )
        error: str = f"Err({INTERNAL_ERROR}({_message(arguments)}))"
        edits.append(
            Edit(
                start=token.start,
                end=significant[close].end,
                replacement=error if tail else f"return {error}",
            )
        )
    return edits


def _is_unreachable(*, significant: list[Token], position: int) -> bool:
    name, bang, paren = significant[position : position + 3]
    return (
        name.kind == IDENT
        and name.text == UNREACHABLE
        and bang.text == BANG
        and paren.text == OPEN_PAREN
    )


def _message(arguments: str) -> str:
    if not arguments:
        return f'"{UNREACHABLE_MESSAGE}"'
    if ARGUMENT_SEPARATOR in arguments:
        return f"format!({arguments.rstrip(ARGUMENT_SEPARATOR)})"
    return arguments


def function_bodies(*, text: str, significant: list[Token]) -> list[tuple[int, int, bool]]:
    bodies: list[tuple[int, int, bool]] = []
    for position, token in enumerate(significant):
        if token.kind != IDENT or token.text != FN:
            continue
        brace: int = position
        while brace < len(significant) and significant[brace].text not in {OPEN_BRACE, SEMICOLON}:
            brace += 1
        if brace == len(significant) or significant[brace].text == SEMICOLON:
            continue
        signature: str = text[token.start : significant[brace].start]
        close: int = matching(significant=significant, at=brace)
        returns_result: bool = re.search(RESULT_RETURN, signature) is not None
        bodies.append((significant[brace].start, significant[close].end, returns_result))
    return bodies


def matching(*, significant: list[Token], at: int) -> int:
    depth: int = 0
    for position in range(at, len(significant)):
        token: Token = significant[position]
        if token.kind == PUNCT and token.text in OPENING_BRACKETS:
            depth += 1
        elif token.kind == PUNCT and token.text in CLOSING_BRACKETS:
            depth -= 1
            if depth == 0:
                return position
    return len(significant) - 1
