"""`use` declarations as flat leaves: each imported path, its local name, or a glob."""

from __future__ import annotations

from scripts.fensu.constants import (
    AS,
    CLOSE_BRACE,
    COLON,
    COMMA,
    GLOB,
    IDENT,
    OPEN_BRACE,
    PUNCT,
    SELF,
    USE,
    WHITESPACE,
)
from scripts.fensu.models import Token, UseLeaf


def use_leaves(*, tokens: list[Token]) -> list[UseLeaf]:
    """The leaves of one `use` declaration, given its tokens from `use` to `;`."""
    significant: list[Token] = [token for token in tokens if token.kind != WHITESPACE]
    position: int = 0
    while significant[position].text != USE:
        position += 1
    return _tree(tokens=significant, position=position + 1, prefix=())[1]


def _tree(
    *, tokens: list[Token], position: int, prefix: tuple[str, ...]
) -> tuple[int, list[UseLeaf]]:
    segments: list[str] = list(prefix)
    leaves: list[UseLeaf] = []
    while position < len(tokens):
        token: Token = tokens[position]
        if token.kind == PUNCT and token.text == OPEN_BRACE:
            position += 1
            while not (tokens[position].kind == PUNCT and tokens[position].text == CLOSE_BRACE):
                nested: list[UseLeaf]
                position, nested = _tree(tokens=tokens, position=position, prefix=tuple(segments))
                leaves.extend(nested)
                if tokens[position].kind == PUNCT and tokens[position].text == COMMA:
                    position += 1
            return position + 1, leaves
        if token.kind == PUNCT and token.text == GLOB:
            leaves.append(UseLeaf(path=tuple(segments), name=GLOB, glob=True))
            return position + 1, leaves
        if token.kind == PUNCT and token.text == COLON:
            position += 1
            continue
        if token.kind == IDENT:
            if token.text == AS:
                alias: str = tokens[position + 1].text
                leaves.append(UseLeaf(path=tuple(segments), name=alias, glob=False))
                return position + 2, leaves
            segments.append(token.text)
            position += 1
            continue
        break
    if segments and segments[-1] == SELF:
        segments = segments[:-1]
    leaves.append(UseLeaf(path=tuple(segments), name=segments[-1], glob=False))
    return position, leaves
