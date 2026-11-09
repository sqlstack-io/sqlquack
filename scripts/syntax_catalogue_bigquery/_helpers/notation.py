"""Rewrite the reference's syntax notation into the catalogue's: `<placeholder>`, KEYWORD, `"<"`."""

from __future__ import annotations

import re

from scripts.grammar_catalogue._helpers.grammar import normalize
from scripts.syntax_catalogue_bigquery.constants import (
    ANGLE_LITERALS,
    BRACKET_CLOSERS,
    BRACKET_OPENERS,
    CALL_LINE,
    DEFINITION_HEAD,
    DEFINITION_INLINE,
    DOC_BACKQUOTED,
    DOC_NAMING_KINDS,
    DOC_PUNCT,
    DOC_TOKEN,
    DOC_WORD,
    GRAMMAR_LIKE,
    HASH_COMMENT,
    INTRODUCERS,
    LETTER_OPERAND,
    LINE_COMMENT,
    LINKED_PLACEHOLDER,
    MARKDOWN_LINK,
    NAMED_ARROW,
    NOT_DEFINITION_HEADS,
    NUMBERED_LINE,
    OBJECT_VERBS,
    OPTION_EQUALS,
    OPTIONS_OPEN,
    PAREN_CLOSE,
    PAREN_OPEN,
    QUOTE_CHARS,
    SQL_COMMENT,
    STATEMENT_END,
)


def strip_comment(line: str) -> str:
    """The line without a trailing `--` or `#` comment outside quotes."""
    quote: str = ""
    for index, ch in enumerate(line):
        if quote:
            quote = "" if ch == quote else quote
        elif ch in QUOTE_CHARS:
            quote = ch
        elif line.startswith(LINE_COMMENT, index) or (
            ch == HASH_COMMENT and not line[:index].strip()
        ):
            return line[:index]
    return line


def unlinked(text: str) -> str:
    """Markdown links as their text; a bare reference link (`#from_clause`) as its anchor's name."""
    text = MARKDOWN_LINK.sub(lambda m: m.group(1), text)
    return LINKED_PLACEHOLDER.sub(lambda m: m.group(1) or m.group(2).replace("-", "_"), text)


class _Converter:
    """Converts one production's tokens, tracking bracket depth and `OPTIONS ( ... )` lists."""

    def __init__(self, *, text: str, placeholders: frozenset[str]) -> None:
        self.pieces: list[tuple[str, str]] = [
            (match.lastgroup or "", match.group(0)) for match in DOC_TOKEN.finditer(text)
        ]
        self.placeholders: frozenset[str] = placeholders
        self.depth: int = 0
        self.options: list[int] = []

    def track(self, *, piece: str, before: str) -> None:
        """Follow the bracket depth and whether the piece sits inside an OPTIONS list."""
        if piece == PAREN_OPEN:
            self.depth += 1
            if before == OPTIONS_OPEN:
                self.options.append(self.depth)
        elif piece == PAREN_CLOSE:
            if self.options and self.options[-1] == self.depth:
                self.options.pop()
            self.depth -= 1

    def is_placeholder(self, *, piece: str, before: str, after: tuple[str, str]) -> bool:
        """True when a word names a value: lower case, a one-letter operand or a listed name."""
        introduces: bool = (
            after[0] in DOC_NAMING_KINDS or after[1] in INTRODUCERS or before in OBJECT_VERBS
        )
        option_name: bool = bool(self.options) and after[1] == OPTION_EQUALS
        if after[1] == NAMED_ARROW or option_name:
            return False
        return (
            any(ch.islower() for ch in piece)
            or LETTER_OPERAND.match(piece) is not None
            or (piece in self.placeholders and not introduces)
        )

    def converted(self) -> str:
        """The production in catalogue notation."""
        out: list[str] = []
        for index, (kind, piece) in enumerate(self.pieces):
            following: bool = index + 1 < len(self.pieces)
            after: tuple[str, str] = self.pieces[index + 1] if following else ("", "")
            before: str = self.pieces[index - 1][1] if index else ""
            self.track(piece=piece, before=before)
            if kind == DOC_BACKQUOTED or (
                kind == DOC_WORD and self.is_placeholder(piece=piece, before=before, after=after)
            ):
                out.append(f"<{piece}>")
            elif kind == DOC_PUNCT and piece in ANGLE_LITERALS:
                out.append(f'"{piece}"')
            else:
                out.append(piece)
        return normalize(" ".join(out))


def convert(*, text: str, placeholders: frozenset[str] = frozenset()) -> str:
    """One production in catalogue notation: words that name values become `<placeholder>`s."""
    return _Converter(text=text, placeholders=placeholders).converted()


def definition_head(line: str) -> tuple[str, str] | None:
    """(name, rest) when the line opens a `name:`, `name :=` or `name ::=` definition."""
    match: re.Match[str] | None = DEFINITION_HEAD.match(line) or DEFINITION_INLINE.match(line)
    if match is None or match.group(1) in NOT_DEFINITION_HEADS:
        return None
    return match.group(1), match.group(2).strip()


def depth_change(line: str) -> int:
    """Opening minus closing brackets outside quotes."""
    bare: str = re.sub(r"'[^'\n]*'|\"[^\"\n]*\"|`[^`\n]*`", "", line)
    return sum(ch in BRACKET_OPENERS for ch in bare) - sum(ch in BRACKET_CLOSERS for ch in bare)


def first_word(line: str) -> str:
    """The line's first word, or "" when it starts with a bracket or symbol."""
    match: re.Match[str] | None = re.match(r"\s*([A-Za-z_][A-Za-z0-9_]*)\b", line)
    return match.group(1) if match else ""


def productions(block: str) -> list[tuple[str, str]]:
    """(definition name or "", text) per production; blank lines, heads and new calls split them."""
    found: list[tuple[str, list[str]]] = []
    current: list[str] | None = None
    depth: int = 0
    for raw in unlinked(block).splitlines():
        line: str = strip_comment(raw).rstrip()
        numbered: re.Match[str] | None = NUMBERED_LINE.match(line)
        if numbered:
            line, current = numbered.group(1), None
        if not line.strip():
            current = None
            continue
        head: tuple[str, str] | None = definition_head(line)
        restarts: bool = (
            bool(current)
            and depth <= 0
            and bool(first_word(line))
            and (
                (bool(CALL_LINE.match(line)) and bool(CALL_LINE.match(current[0])))
                or (
                    len(current) == 1
                    and first_word(line) == first_word(current[0])
                    and any(ch.islower() for ch in first_word(line))
                )
            )
        )
        if head is not None:
            found.append((head[0], [head[1]] if head[1] else []))
            current, depth = found[-1][1], depth_change(head[1])
            continue
        if current is None or restarts:
            found.append(("", []))
            current, depth = found[-1][1], 0
        current.append(line)
        depth += depth_change(line)
    out: list[tuple[str, str]] = []
    for name, lines in found:
        text: str = normalize(" ".join(lines))
        text = STATEMENT_END.sub("", text)
        if text:
            out.append((name, text))
    return out


def grammar_like(text: str) -> bool:
    """Whether a block written after prose still reads as notation rather than an example."""
    bare: str = SQL_COMMENT.sub(" ", re.sub(r"'[^'\n]*'|\"[^\"\n]*\"", " ", text))
    return bool(GRAMMAR_LIKE.search(bare))
