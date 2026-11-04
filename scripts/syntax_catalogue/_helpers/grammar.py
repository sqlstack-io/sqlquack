"""Text operations on documented grammar: normalizing, splitting and naming productions."""

from __future__ import annotations

import hashlib
import re

from scripts.syntax_catalogue.constants import (
    CALL_START,
    CAST_OPERATOR,
    CLOSE_OPTIONAL,
    CLOSE_PAREN,
    CLOSERS,
    DEFINITION,
    LEAD_WORDS,
    NAME_ALTERNATION,
    NAMED_ARROW,
    OPEN_OPTIONAL,
    OPEN_PAREN,
    OPENERS,
    QUOTES,
    QUOTED_OR_ANGLE,
    QUOTED_OR_PLACEHOLDER,
    SPECIAL_KIND_CAST,
    SPECIAL_KIND_NAMED,
    SPECIAL_KIND_NONCALL,
    SPECIAL_KIND_OVER,
    SPECIAL_KIND_WITHIN,
    SPECIAL_WORDS,
    SYMBOLS,
    WORD,
)


def sha256(text: str) -> str:
    """The hex SHA-256 of a text's UTF-8 bytes."""
    return hashlib.sha256(text.encode()).hexdigest()


def slug(text: str) -> str:
    """Lowercase words joined by single hyphens."""
    return re.sub(r"-+", "-", re.sub(r"[^a-z0-9]+", "-", text.lower())).strip("-")


def normalize(text: str) -> str:
    """The grammar on one line with single spaces: the normalized grammar the catalogue tracks."""
    return re.sub(r"\s+", " ", text).strip()


def strip_comments(text: str) -> str:
    """The block without SQL comments."""
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    return "\n".join(re.sub(r"(^|\s)--.*$", "", line) for line in text.splitlines())


def bracket_depth_change(line: str) -> int:
    """Opening minus closing brackets outside quotes and placeholders."""
    bare: str = QUOTED_OR_ANGLE.sub("", line)
    return sum(1 for ch in bare if ch in OPENERS) - sum(1 for ch in bare if ch in CLOSERS)


def statement_lead(line: str) -> tuple[str, ...]:
    """The first two keywords of a line, or an empty tuple when it does not start with two."""
    words: list[str] = bare_words(line)[:LEAD_WORDS]
    if len(words) < LEAD_WORDS or not all(w.isupper() for w in words):
        return ()
    return tuple(words)


def split_productions(block: str) -> list[str]:
    """Split a block where brackets close: at blank lines, `name ::=` or a repeated lead."""
    out: list[str] = []
    current: list[str] = []
    depth: int = 0
    lead: tuple[str, ...] = ()
    for raw in strip_comments(block).splitlines():
        line: str = raw.rstrip()
        repeats: bool = bool(lead) and statement_lead(line) == lead
        if (not line.strip() or DEFINITION.match(line) or repeats) and depth <= 0 and current:
            out.append("\n".join(current))
            current, depth = [], 0
        if not current:
            lead = statement_lead(line)
        if not line.strip():
            continue
        current.append(line)
        depth += bracket_depth_change(line)
    if current:
        out.append("\n".join(current))
    normalized: list[str] = [normalize(chunk) for chunk in out]
    return [chunk for chunk in normalized if chunk]


def bare_words(grammar: str) -> list[str]:
    """Words outside placeholders and quotes, in order."""
    return WORD.findall(QUOTED_OR_PLACEHOLDER.sub(" ", grammar))


def required_skeleton(grammar: str) -> str:
    """The grammar without optional `[ ... ]` parts, placeholders and quoted text."""
    out: list[str] = []
    depth: int = 0
    for ch in QUOTED_OR_PLACEHOLDER.sub(" ", grammar):
        if ch == OPEN_OPTIONAL:
            depth += 1
        elif ch == CLOSE_OPTIONAL:
            depth = max(0, depth - 1)
        elif depth == 0:
            out.append(ch)
    return normalize("".join(out))


def call_name(grammar: str) -> str:
    """The upper-cased function name a call grammar starts with, or an empty string."""
    match: re.Match[str] | None = CALL_START.match(grammar)
    return match.group(1).upper() if match else ""


def call_end(grammar: str, *, start: int) -> int:
    """The index just past the parenthesis that closes the one opened at or after start."""
    depth: int = 0
    index: int = start
    quote: str = ""
    while index < len(grammar):
        ch: str = grammar[index]
        if quote:
            quote = "" if ch == quote else quote
        elif ch in QUOTES:
            quote = ch
        elif ch == OPEN_PAREN:
            depth += 1
        elif ch == CLOSE_PAREN:
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1
    return -1


def split_calls(grammar: str) -> list[str]:
    """The calls of a grammar that lists complete calls one after another, else the grammar."""
    calls: list[str] = []
    rest: str = grammar.strip()
    while rest:
        match: re.Match[str] | None = CALL_START.match(rest)
        if match is None or (calls and match.group(1).upper() in SPECIAL_WORDS):
            return [grammar]
        end: int = call_end(rest, start=match.end() - 1)
        if end < 0:
            return [grammar]
        calls.append(rest[:end].strip())
        rest = rest[end:].lstrip().removeprefix(";").lstrip()
    return calls if len(calls) > 1 else [grammar]


def expand_name_alternation(grammar: str) -> list[str]:
    """One grammar per function name for `{ NAME1 | NAME2 } ( ... )`, else the grammar."""
    match: re.Match[str] | None = NAME_ALTERNATION.match(grammar)
    if match is None:
        return [grammar]
    names: list[str] = [name.strip() for name in match.group(1).split("|")]
    tail: str = grammar[match.end() :]
    return [f"{name}{tail}" for name in names]


def special_kind(grammar: str) -> str:
    """The construct that gives a call its own grammar, or an empty string for a plain call."""
    if NAMED_ARROW in grammar:
        return SPECIAL_KIND_NAMED
    if CAST_OPERATOR in grammar:
        return SPECIAL_KIND_CAST
    match: re.Match[str] | None = CALL_START.match(grammar)
    if match is None:
        return SPECIAL_KIND_NONCALL
    words: list[str] = [w for w in bare_words(grammar[match.end() :]) if w.isupper()]
    for kind in (SPECIAL_KIND_OVER, SPECIAL_KIND_WITHIN):
        if kind in words:
            return kind
    keywords: list[str] = [w for w in words if w in SPECIAL_WORDS]
    return keywords[0] if keywords else ""


def symbol_name(grammar: str) -> str:
    """A name for a form with no words outside placeholders, from its operator symbols."""
    text: str = QUOTED_OR_ANGLE.sub(" ", grammar)
    names: list[str] = []
    for symbol, name in SYMBOLS:
        if symbol in text:
            names.append(name)
            text = text.replace(symbol, " ")
    return "-".join(names)
