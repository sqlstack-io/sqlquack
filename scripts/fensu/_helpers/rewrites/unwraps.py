"""Edits turning `.unwrap()` and `.expect(..)` in `Result`-returning functions into `?`."""

from __future__ import annotations

from scripts.fensu._helpers.rewrites.unreachable import function_bodies, matching
from scripts.fensu.constants import (
    DOT,
    EXPECT,
    IDENT,
    INTERNAL_ERROR,
    OPEN_PAREN,
    TRIVIA_KINDS,
    UNWRAP,
    UNWRAP_MESSAGE,
)
from scripts.fensu.models import Edit, Token


def unwrap_edits(*, text: str, tokens: list[Token]) -> list[Edit]:
    """Edits rewriting each `.unwrap()`/`.expect(..)` whose function returns a `Result`."""
    significant: list[Token] = [token for token in tokens if token.kind not in TRIVIA_KINDS]
    bodies: list[tuple[int, int, bool]] = function_bodies(text=text, significant=significant)
    edits: list[Edit] = []
    for position, token in enumerate(significant[:-2]):
        if not _is_unwrap(significant=significant, position=position):
            continue
        enclosing: list[tuple[int, int, bool]] = [
            body for body in bodies if body[0] < token.start < body[1]
        ]
        if not enclosing or not max(enclosing, key=lambda body: body[0])[2]:
            continue
        close: int = matching(significant=significant, at=position + 1)
        message: str = text[significant[position + 1].end : significant[close].start].strip()
        edits.append(
            Edit(
                start=significant[position].start,
                end=significant[close].end,
                replacement=f"ok_or_else(|| {INTERNAL_ERROR}({message or UNWRAP_MESSAGE}))?",
            )
        )
    return edits


def _is_unwrap(*, significant: list[Token], position: int) -> bool:
    name, paren = significant[position : position + 2]
    previous: Token = significant[position - 1]
    return (
        previous.text == DOT
        and name.kind == IDENT
        and name.text in {UNWRAP, EXPECT}
        and paren.text == OPEN_PAREN
    )
