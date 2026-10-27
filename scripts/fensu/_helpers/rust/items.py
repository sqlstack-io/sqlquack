"""Top-level Rust items of a source region: their kinds, names, visibility and exact text ranges."""

from __future__ import annotations

from scripts.fensu.constants import (
    BANG,
    BRACED_ITEM_KEYWORDS,
    CLOSE_ANGLE,
    CLOSE_BRACE,
    CLOSING_BRACKETS,
    COMMENT,
    CONST,
    DECLARATION_KEYWORDS,
    DOC_COMMENT,
    FN_MODIFIERS,
    HASH,
    IDENT,
    IMPL,
    IMPL_HEADER_ENDS,
    INNER_ATTRIBUTE,
    INNER_DOC,
    INNER_DOC_PREFIXES,
    INNER_GROUP_CLOSERS,
    INNER_GROUP_OPENERS,
    ITEM_MODIFIERS,
    MACRO,
    MACRO_CLOSERS,
    MACRO_RULES,
    MOD,
    NON_DOC_COMMENT_KINDS,
    OPEN_ANGLE,
    OPEN_BRACE,
    OPEN_PAREN,
    OPENING_BRACKETS,
    PUB,
    PUNCT,
    SEMICOLON,
    STRING,
    USE,
    WHERE,
    WHITESPACE,
)
from scripts.fensu.exceptions import TransformError
from scripts.fensu.models import Item, Token


def parse_items(*, text: str, tokens: list[Token], start: int, end: int) -> list[Item]:
    """The items of `text[start:end]` (a file, or the inside of an `impl` or `mod` body)."""
    indices: list[int] = [i for i, token in enumerate(tokens) if start <= token.start < end]
    significant: list[int] = [i for i in indices if tokens[i].kind != WHITESPACE]
    items: list[Item] = []
    position: int = 0
    while position < len(significant):
        item: Item
        item, position = _parse_item(text=text, tokens=tokens, significant=significant, at=position)
        items.append(item)
    return items


def _parse_item(
    *, text: str, tokens: list[Token], significant: list[int], at: int
) -> tuple[Item, int]:
    first: Token = tokens[significant[at]]
    if first.kind == DOC_COMMENT and first.text.startswith(INNER_DOC_PREFIXES):
        return _item(kind=INNER_DOC, start=first.start, end=first.end), at + 1
    if first.kind in NON_DOC_COMMENT_KINDS:
        return _item(kind=COMMENT, start=first.start, end=first.end), at + 1
    header, attributes, inner_end = _leading(
        text=text, tokens=tokens, significant=significant, at=at
    )
    if inner_end is not None:
        return _item(
            kind=INNER_ATTRIBUTE,
            start=first.start,
            end=_end_of(tokens=tokens, significant=significant, at=inner_end),
        ), inner_end + 1
    visibility, keyword_at = _visibility(
        text=text, tokens=tokens, significant=significant, at=header
    )
    kind, name, trait_name, end_at, body = _shape(
        text=text, tokens=tokens, significant=significant, at=keyword_at
    )
    return (
        Item(
            kind=kind,
            name=name,
            start=first.start,
            end=_end_of(tokens=tokens, significant=significant, at=end_at),
            header_start=tokens[significant[header]].start,
            keyword_start=tokens[significant[keyword_at]].start,
            visibility=visibility,
            trait_name=trait_name,
            body_start=body[0] if body else -1,
            body_end=body[1] if body else -1,
            attributes=tuple(attributes),
        ),
        end_at + 1,
    )


def _leading(
    *, text: str, tokens: list[Token], significant: list[int], at: int
) -> tuple[int, list[str], int | None]:
    position: int = at
    attributes: list[str] = []
    while position < len(significant):
        token: Token = tokens[significant[position]]
        if token.kind == DOC_COMMENT:
            position += 1
        elif _punct(token=token, text=HASH):
            inner: bool = _punct(token=tokens[significant[position + 1]], text=BANG)
            close: int = _matching(
                tokens=tokens, significant=significant, at=position + (2 if inner else 1)
            )
            if inner:
                return position, attributes, close
            attributes.append(text[token.start : tokens[significant[close]].end])
            position = close + 1
        else:
            break
    return position, attributes, None


def _visibility(
    *, text: str, tokens: list[Token], significant: list[int], at: int
) -> tuple[str, int]:
    visibility: str = ""
    position: int = at
    token: Token = tokens[significant[position]]
    if token.kind == IDENT and token.text == PUB:
        if _punct(token=tokens[significant[position + 1]], text=OPEN_PAREN):
            close: int = _matching(tokens=tokens, significant=significant, at=position + 1)
            visibility = text[token.start : tokens[significant[close]].end]
            position = close + 1
        else:
            visibility = PUB
            position += 1
    while (
        tokens[significant[position]].text in ITEM_MODIFIERS
        or tokens[significant[position]].kind == STRING
    ):
        following: Token = tokens[significant[position + 1]]
        if tokens[significant[position]].text == CONST and following.text not in FN_MODIFIERS:
            break
        position += 1
    return visibility, position


def _shape(
    *, text: str, tokens: list[Token], significant: list[int], at: int
) -> tuple[str, str, str, int, tuple[int, int] | None]:
    keyword: str = tokens[significant[at]].text
    if keyword == IMPL:
        end_at, body, name, trait_name = _impl_header(
            text=text, tokens=tokens, significant=significant, at=at
        )
        return IMPL, name, trait_name, end_at, body
    if keyword == MACRO_RULES:
        end_at: int = _block_end(tokens=tokens, significant=significant, at=at)
        if tokens[significant[end_at]].text in MACRO_CLOSERS:
            end_at += 1
        return MACRO_RULES, tokens[significant[at + 2]].text, "", end_at, None
    if keyword in DECLARATION_KEYWORDS:
        name: str = "" if keyword == USE else tokens[significant[at + 1]].text
        end_at = _item_end(tokens=tokens, significant=significant, at=at)
        body: tuple[int, int] | None = None
        if keyword == MOD and _punct(token=tokens[significant[end_at]], text=CLOSE_BRACE):
            opening: int = _first_brace(tokens=tokens, significant=significant, at=at)
            body = (tokens[significant[opening]].end, tokens[significant[end_at]].start)
        return keyword, name, "", end_at, body
    if tokens[significant[at]].kind == IDENT and _punct(
        token=tokens[significant[at + 1]], text=BANG
    ):
        close: int = _matching(tokens=tokens, significant=significant, at=at + 2)
        ends_with_semicolon: bool = close + 1 < len(significant) and _punct(
            token=tokens[significant[close + 1]], text=SEMICOLON
        )
        return MACRO, keyword, "", close + 1 if ends_with_semicolon else close, None
    start: int = tokens[significant[at]].start
    raise TransformError(f"unrecognised item at offset {start}: {text[start : start + 60]!r}")


def _item(*, kind: str, start: int, end: int) -> Item:
    return Item(
        kind=kind,
        name="",
        start=start,
        end=end,
        header_start=start,
        keyword_start=start,
        visibility="",
        trait_name="",
        body_start=-1,
        body_end=-1,
        attributes=(),
    )


def _end_of(*, tokens: list[Token], significant: list[int], at: int) -> int:
    return tokens[significant[at]].end


def _punct(*, token: Token, text: str) -> bool:
    return token.kind == PUNCT and token.text == text


def _matching(*, tokens: list[Token], significant: list[int], at: int) -> int:
    depth: int = 0
    position: int = at
    while position < len(significant):
        token: Token = tokens[significant[position]]
        if token.kind == PUNCT and token.text in OPENING_BRACKETS:
            depth += 1
        elif token.kind == PUNCT and token.text in CLOSING_BRACKETS:
            depth -= 1
            if depth == 0:
                return position
        position += 1
    raise TransformError("unbalanced brackets")


def _first_brace(*, tokens: list[Token], significant: list[int], at: int) -> int:
    depth: int = 0
    position: int = at
    while position < len(significant):
        token: Token = tokens[significant[position]]
        if token.kind == PUNCT and token.text in INNER_GROUP_OPENERS:
            depth += 1
        elif token.kind == PUNCT and token.text in INNER_GROUP_CLOSERS:
            depth -= 1
        elif depth == 0 and _punct(token=token, text=OPEN_BRACE):
            return position
        position += 1
    raise TransformError("no body")


def _block_end(*, tokens: list[Token], significant: list[int], at: int) -> int:
    position: int = at
    while not (
        tokens[significant[position]].kind == PUNCT
        and tokens[significant[position]].text in OPENING_BRACKETS
    ):
        position += 1
    return _matching(tokens=tokens, significant=significant, at=position)


def _item_end(*, tokens: list[Token], significant: list[int], at: int) -> int:
    depth: int = 0
    position: int = at
    while position < len(significant):
        token: Token = tokens[significant[position]]
        if token.kind == PUNCT and token.text in OPENING_BRACKETS:
            depth += 1
        elif token.kind == PUNCT and token.text in CLOSING_BRACKETS:
            depth -= 1
            if (
                depth == 0
                and token.text == CLOSE_BRACE
                and _ends_at_brace(tokens=tokens, significant=significant, at=at)
            ):
                return position
        elif depth == 0 and _punct(token=token, text=SEMICOLON):
            return position
        position += 1
    raise TransformError("unterminated item")


def _ends_at_brace(*, tokens: list[Token], significant: list[int], at: int) -> bool:
    keyword: str = tokens[significant[at]].text
    return keyword in BRACED_ITEM_KEYWORDS


def _impl_header(
    *, text: str, tokens: list[Token], significant: list[int], at: int
) -> tuple[int, tuple[int, int], str, str]:
    opening: int = _first_brace(tokens=tokens, significant=significant, at=at)
    close: int = _matching(tokens=tokens, significant=significant, at=opening)
    header: list[Token] = [tokens[i] for i in significant[at + 1 : opening]]
    position: int = 0
    if header and _punct(token=header[0], text=OPEN_ANGLE):
        depth: int = 0
        while position < len(header):
            if _punct(token=header[position], text=OPEN_ANGLE):
                depth += 1
            elif _punct(token=header[position], text=CLOSE_ANGLE):
                depth -= 1
                if depth == 0:
                    position += 1
                    break
            position += 1
    rest: list[Token] = header[position:]
    for_at: int | None = None
    depth = 0
    for index, token in enumerate(rest):
        if _punct(token=token, text=OPEN_ANGLE):
            depth += 1
        elif _punct(token=token, text=CLOSE_ANGLE):
            depth -= 1
        elif depth == 0 and token.kind == IDENT and token.text in IMPL_HEADER_ENDS:
            if token.text == WHERE:
                rest = rest[:index]
                break
            for_at = index
            break
    trait_part: list[Token] = rest[:for_at] if for_at is not None else []
    type_part: list[Token] = rest[for_at + 1 :] if for_at is not None else rest
    for index, token in enumerate(type_part):
        if token.kind == IDENT and token.text == WHERE:
            type_part = type_part[:index]
            break
    return (
        close,
        (tokens[significant[opening]].end, tokens[significant[close]].start),
        _path_name(type_part),
        _path_name(trait_part),
    )


def _path_name(tokens: list[Token]) -> str:
    depth: int = 0
    name: str = ""
    for token in tokens:
        if _punct(token=token, text=OPEN_ANGLE):
            depth += 1
        elif _punct(token=token, text=CLOSE_ANGLE):
            depth -= 1
        elif depth == 0 and token.kind == IDENT:
            name = token.text
    return name
