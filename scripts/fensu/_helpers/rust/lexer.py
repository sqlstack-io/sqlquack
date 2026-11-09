"""A Rust lexer exact enough to tell comments, strings and code apart."""

from __future__ import annotations

from collections.abc import Callable

from scripts.fensu.constants import (
    BACKSLASH,
    BLOCK_COMMENT,
    BLOCK_COMMENT_CLOSE,
    BLOCK_COMMENT_OPEN,
    BYTE_CHAR_PREFIX,
    CHAR,
    DOC_COMMENT,
    DOT,
    DOUBLE_QUOTE,
    EMPTY_BLOCK_COMMENT,
    HASH,
    IDENT,
    INNER_BLOCK_DOC,
    INNER_LINE_DOC,
    LIFETIME,
    LINE_COMMENT,
    LINE_COMMENT_OPEN,
    NOT_BLOCK_DOC,
    NOT_LINE_DOC,
    NUMBER,
    OUTER_BLOCK_DOC,
    OUTER_LINE_DOC,
    PUNCT,
    QUOTED_STRING_PREFIXES,
    RAW_IDENTIFIER_PREFIX,
    RAW_STRING_PREFIXES,
    SINGLE_QUOTE,
    STRING,
    UNDERSCORE,
    WHITESPACE,
)
from scripts.fensu.models import Token


def tokenize(*, text: str) -> list[Token]:
    """Every token of `text`, trivia included, so the tokens joined back equal the text."""
    tokens: list[Token] = []
    position: int = 0
    while position < len(text):
        kind: str
        end: int
        kind, end = _next_token(text=text, start=position)
        tokens.append(Token(kind=kind, start=position, end=end, text=text[position:end]))
        position = end
    return tokens


def is_doc_comment(*, text: str) -> bool:
    """Whether a comment's text is a doc comment (`///`, `//!`, `/**`, `/*!`)."""
    if text.startswith(LINE_COMMENT_OPEN):
        outer: bool = text.startswith(OUTER_LINE_DOC) and not text.startswith(NOT_LINE_DOC)
        return outer or text.startswith(INNER_LINE_DOC)
    if text.startswith(OUTER_BLOCK_DOC):
        return not text.startswith(NOT_BLOCK_DOC) and text != EMPTY_BLOCK_COMMENT
    return text.startswith(INNER_BLOCK_DOC)


def _next_token(*, text: str, start: int) -> tuple[str, int]:
    character: str = text[start]
    if character.isspace():
        return WHITESPACE, _while(text=text, start=start, accept=str.isspace)
    if text.startswith(LINE_COMMENT_OPEN, start):
        end: int = text.find("\n", start)
        end = len(text) if end < 0 else end
        return _comment_kind(text=text[start:end], plain=LINE_COMMENT), end
    if text.startswith(BLOCK_COMMENT_OPEN, start):
        end = _block_comment_end(text=text, start=start)
        return _comment_kind(text=text[start:end], plain=BLOCK_COMMENT), end
    literal: int | None = _prefixed_literal_end(text=text, start=start)
    if literal is not None:
        return STRING, literal
    if character == DOUBLE_QUOTE:
        return STRING, _quoted_end(text=text, start=start + 1, quote=DOUBLE_QUOTE)
    if character == SINGLE_QUOTE:
        return _quote_token(text=text, start=start)
    if character == UNDERSCORE or character.isalpha():
        return IDENT, _ident_end(text=text, start=start)
    if character.isdigit():
        return NUMBER, _number_end(text=text, start=start)
    return PUNCT, start + 1


def _comment_kind(*, text: str, plain: str) -> str:
    return DOC_COMMENT if is_doc_comment(text=text) else plain


def _while(*, text: str, start: int, accept: Callable[[str], bool]) -> int:
    position: int = start
    while position < len(text) and accept(text[position]):
        position += 1
    return position


def _block_comment_end(*, text: str, start: int) -> int:
    depth: int = 0
    position: int = start
    while position < len(text):
        if text.startswith(BLOCK_COMMENT_OPEN, position):
            depth += 1
            position += 2
        elif text.startswith(BLOCK_COMMENT_CLOSE, position):
            depth -= 1
            position += 2
            if depth == 0:
                return position
        else:
            position += 1
    return len(text)


def _prefixed_literal_end(*, text: str, start: int) -> int | None:
    for prefix in RAW_STRING_PREFIXES:
        if text.startswith(prefix, start):
            hashes: int = _while(text=text, start=start + len(prefix), accept=_is_hash)
            if hashes < len(text) and text[hashes] == DOUBLE_QUOTE:
                closing: str = DOUBLE_QUOTE + HASH * (hashes - start - len(prefix))
                end: int = text.find(closing, hashes + 1)
                return len(text) if end < 0 else end + len(closing)
    for prefix in QUOTED_STRING_PREFIXES:
        if text.startswith(prefix + DOUBLE_QUOTE, start):
            return _quoted_end(text=text, start=start + 2, quote=DOUBLE_QUOTE)
    if text.startswith(BYTE_CHAR_PREFIX, start):
        return _quoted_end(text=text, start=start + 2, quote=SINGLE_QUOTE)
    return None


def _is_hash(character: str) -> bool:
    return character == HASH


def _is_ident_character(character: str) -> bool:
    return character == UNDERSCORE or character.isalnum()


def _quoted_end(*, text: str, start: int, quote: str) -> int:
    position: int = start
    while position < len(text):
        if text[position] == BACKSLASH:
            position += 2
        elif text[position] == quote:
            return position + 1
        else:
            position += 1
    return len(text)


def _quote_token(*, text: str, start: int) -> tuple[str, int]:
    if start + 1 < len(text) and text[start + 1] == BACKSLASH:
        return CHAR, _quoted_end(text=text, start=start + 1, quote=SINGLE_QUOTE)
    if start + 2 < len(text) and text[start + 2] == SINGLE_QUOTE:
        return CHAR, start + 3
    return LIFETIME, _ident_end(text=text, start=start + 1)


def _ident_end(*, text: str, start: int) -> int:
    position: int = start
    if text.startswith(RAW_IDENTIFIER_PREFIX, position):
        position += len(RAW_IDENTIFIER_PREFIX)
    return _while(text=text, start=position, accept=_is_ident_character)


def _number_end(*, text: str, start: int) -> int:
    position: int = start
    while position < len(text):
        character: str = text[position]
        if _is_ident_character(character):
            position += 1
        elif (
            character == DOT
            and position + 1 < len(text)
            and text[position + 1].isdigit()
            and DOT not in text[start:position]
        ):
            position += 1
        else:
            break
    return position
