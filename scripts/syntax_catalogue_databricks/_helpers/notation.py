"""Turn the reference's syntax blocks into the catalogue grammar notation."""

from __future__ import annotations

import re

from scripts.grammar_catalogue._helpers.grammar import normalize
from scripts.syntax_catalogue_databricks.constants import (
    CALL_OPEN,
    CHOICE_BAR,
    CLAUSE_WORD,
    CLAUSE_WORDS,
    CLOSING,
    DEFINITION_HEAD_LINE,
    DOC_TOKEN,
    ELISION,
    EMPTY_ANGLE_LENGTH,
    KEYWORD_TOKEN,
    LINE_COMMENT,
    LINE_STARTERS,
    NAMED_ARROW,
    OPENING,
    REPEAT_COMMA,
    REPEAT_PLAIN,
    STATEMENT_WORD,
    TYPE_OPENERS,
    UNICODE_ELLIPSIS,
)
from scripts.syntax_catalogue_databricks.models import RawProduction


def snake(word: str) -> str:
    """`elementType` as `element_type`, `SQL_statement` as `sql_statement`."""
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", word).lower()


def depth_change(line: str) -> int:
    """Opening minus closing brackets of a line, outside quotes."""
    bare: str = re.sub(r"'[^']*'|\"[^\"]*\"", "", line)
    return sum(bare.count(ch) for ch in OPENING) - sum(bare.count(ch) for ch in CLOSING)


def raw_productions(text: str) -> list[RawProduction]:
    """The block cut into statements and definitions, comments removed, lines joined."""
    found: list[RawProduction] = []
    name: str = ""
    lines: list[str] = []
    depth: int = 0
    blank: bool = True
    first_indent: int = 0
    for raw in text.replace(UNICODE_ELLIPSIS, ELISION).splitlines():
        line: str = LINE_COMMENT.sub("", raw).rstrip()
        if not line.strip():
            blank = True
            continue
        head: re.Match[str] | None = DEFINITION_HEAD_LINE.match(line)
        indent: int = len(line) - len(line.lstrip())
        if head and not blank and lines and depth <= 0 and indent > first_indent:
            head = None
        restart: bool = bool(head) and blank and indent <= first_indent
        repeats: bool = (
            bool(lines)
            and not name
            and depth <= 0
            and indent <= first_indent
            and _first_word(line) is not None
            and _first_word(line) == _first_word(lines[0])
        )
        if repeats:
            found.append(RawProduction(name=name, text=normalize(" ".join(lines))))
            lines, depth = [], 0
        if (depth <= 0 or restart) and (blank or head) and (head or lines):
            if lines:
                found.append(RawProduction(name=name, text=normalize(" ".join(lines))))
            name, lines, depth = (head.group(1) if head else ""), [], 0
            first_indent = indent
            if head:
                blank = False
                continue
        lines.append(line)
        depth += depth_change(line)
        blank = False
    if lines:
        found.append(RawProduction(name=name, text=normalize(" ".join(lines))))
    return found


def _first_word(line: str) -> str | None:
    """A line's leading statement keyword, or None."""
    match: re.Match[str] | None = STATEMENT_WORD.match(line)
    return match.group(1) if match else None


def split_top(text: str) -> list[str]:
    """A statement's top-level `a | b` alternatives, outside every bracket."""
    parts: list[str] = []
    depth: int = 0
    current: list[str] = []
    for token in DOC_TOKEN.findall(text):
        if token in OPENING:
            depth += 1
        elif token in CLOSING:
            depth -= 1
        if token == CHOICE_BAR and depth == 0:
            parts.append(" ".join(current))
            current = []
            continue
        current.append(token)
    parts.append(" ".join(current))
    return [part for part in parts if part.strip()]


def convert(*, text: str, calls: frozenset[str], keywords: frozenset[str] = frozenset()) -> str:
    """One production in catalogue notation, with the page's function names and keyword words."""
    text = REPEAT_COMMA.sub(" [ , ... ] ", text)
    text = REPEAT_PLAIN.sub(" [ ... ] ", text)
    tokens: list[str] = DOC_TOKEN.findall(text)
    spaced: list[bool] = [False] * len(tokens)
    position: int = 0
    for index, token in enumerate(tokens):
        start: int = text.index(token, position)
        spaced[index] = start > 0 and text[start - 1].isspace()
        position = start + len(token)
    out: list[str] = []
    for index, token in enumerate(tokens):
        following: str = tokens[index + 1] if index + 1 < len(tokens) else ""
        previous: str = tokens[index - 1] if index else ""
        first: bool = index == 0 or previous in LINE_STARTERS
        glued: bool = index + 1 < len(tokens) and not spaced[index + 1]
        call: bool = following == CALL_OPEN and (glued or (first and token.lower() in calls))
        if token == CLAUSE_WORD:
            out.append(clause_name(tokens[:index]))
            continue
        if token.lower() in keywords and token.isalpha():
            out.append(token.upper())
            continue
        out.append(word_text(token=token, following=following, previous=previous, call=call))
    return normalize(" ".join(out))


def clause_name(before: list[str]) -> str:
    """`ORDER BY clause` as `<order_by_clause>`: the keywords before it name the linked clause."""
    words: list[str] = []
    for token in reversed(before):
        if not KEYWORD_TOKEN.fullmatch(token) or len(words) == CLAUSE_WORDS:
            break
        words.insert(0, token.lower())
    return "<" + "_".join([*words, CLAUSE_WORD]) + ">"


def word_text(*, token: str, following: str, previous: str, call: bool) -> str:
    """One token: a keyword, a call name, a `<placeholder>`, a literal or punctuation."""
    if token.startswith("<") and token.endswith(">") and len(token) > EMPTY_ANGLE_LENGTH:
        if previous.upper() in TYPE_OPENERS:
            return f"< <{snake(token[1:-1])}> >"
        return f"<{snake(token[1:-1])}>"
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", token):
        return token
    if not any(ch.islower() for ch in token):
        return token
    if following == NAMED_ARROW:
        return token
    if call:
        return token.upper()
    return f"<{snake(token)}>"


def productions(
    *, text: str, calls: frozenset[str], keywords: frozenset[str] = frozenset()
) -> list[RawProduction]:
    """The block's statements, one per top-level alternative, and definitions, in notation."""
    found: list[RawProduction] = []
    for raw in raw_productions(text):
        if raw.name:
            body: str = convert(text=raw.text, calls=calls, keywords=keywords)
            found.append(RawProduction(name=snake(raw.name), text=body))
            continue
        for part in split_top(raw.text):
            found.append(
                RawProduction(name="", text=convert(text=part, calls=calls, keywords=keywords))
            )
    return found
