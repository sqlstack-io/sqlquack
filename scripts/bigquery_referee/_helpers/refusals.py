"""The pinned parser's refusal messages, read from googlesql.tm, as the syntax error classes."""

from __future__ import annotations

import functools
import hashlib
import json
import re
from pathlib import Path

from scripts.bigquery_referee._helpers.values import rows_of
from scripts.bigquery_referee.constants import (
    BACKSLASH,
    BARE_SYNTAX_PATTERNS,
    COMMA,
    DEPTH_CHANGE,
    DYNAMIC_PART,
    FORMAT_SLOT,
    GENERIC_SYNTAX,
    LITERAL_PATTERN,
    PARSER_GRAMMAR,
    PARSER_GRAMMAR_SHA256,
    PARSER_INTERNAL_CLASSES,
    PARSER_REASON,
    PARSER_REFUSALS_FILE,
    PARSER_REFUSALS_FORMAT,
    PARSER_REFUSALS_READ,
    QUOTE,
    REFUSAL_CALL,
    REFUSAL_CODE,
    STRING_BUILDER,
    STRING_BUILDER_LITERAL,
    SYNTAX_ERROR,
)
from scripts.bigquery_referee.exceptions import RefereeError
from scripts.bigquery_referee.models import ErrorClass
from scripts.bigquery_referee.types import Row


def _string_end(*, text: str, start: int) -> int:
    """The index after the closing quote of the string literal that opens at `start`."""
    index: int = start + 1
    while text[index] != QUOTE:
        index += 2 if text[index] == BACKSLASH else 1
    return index + 1


def _call_arguments(*, text: str, start: int) -> str:
    """The argument text of the call whose opening parenthesis ends just before `start`."""
    depth: int = 1
    index: int = start
    while depth:
        if text[index] == QUOTE:
            index = _string_end(text=text, start=index)
            continue
        depth += DEPTH_CHANGE.get(text[index], 0)
        index += 1
    return text[start : index - 1]


def _message_argument(arguments: str) -> str:
    """The second argument of a MakeSyntaxError call: the message."""
    depth: int = 0
    for index, char in enumerate(arguments):
        depth += DEPTH_CHANGE.get(char, 0)
        if char == COMMA and depth == 0:
            return arguments[index + 1 :].strip()
    return ""


def _literal(token: re.Match[str]) -> str:
    """A C++ string literal's text: a raw literal as written, an ordinary one unescaped."""
    raw: str | None = token.group(1)
    return raw if raw is not None else token.group(2).encode().decode("unicode_escape")


def _pattern(message: str) -> str | None:
    """The message's literals in order, its dynamic parts `.+?`, anchored at the start."""
    tokens: list[re.Match[str]] = list(re.finditer(LITERAL_PATTERN, message, re.S))
    if not tokens:
        return None
    pattern: str = "^"
    built: bool = message.startswith(STRING_BUILDER)
    if built and not re.match(STRING_BUILDER_LITERAL, message[message.index("(") + 1 :]):
        pattern += r"\S+"
    previous: int | None = None
    for token in tokens:
        if previous is not None and message[previous : token.start()].strip():
            pattern += DYNAMIC_PART
        pattern += DYNAMIC_PART.join(re.escape(part) for part in _literal(token).split(FORMAT_SLOT))
        previous = token.end()
    pattern = pattern.replace("\\ ", " ")
    if pattern in BARE_SYNTAX_PATTERNS:
        return None
    return pattern.removesuffix(DYNAMIC_PART)


def refusal_rows(grammar: str) -> list[Row]:
    """One row per distinct refusal message of the grammar, numbered in grammar order."""
    lines: dict[str, int] = {}
    for call in re.finditer(REFUSAL_CALL, grammar):
        arguments: str = _call_arguments(text=grammar, start=call.end())
        pattern: str | None = _pattern(_message_argument(arguments))
        if pattern is not None and pattern not in lines:
            lines[pattern] = grammar[: call.start()].count("\n") + 1
    return [
        {"code": REFUSAL_CODE.format(number=number), "line": line, "pattern": pattern}
        for number, (pattern, line) in enumerate(lines.items(), start=1)
    ]


def write_refusals(grammar_path: Path) -> int:
    """Rewrite parser_refusals.json from the pinned grammar file; a different grammar is refused."""
    data: bytes = grammar_path.read_bytes()
    if hashlib.sha256(data).hexdigest() != PARSER_GRAMMAR_SHA256:
        raise RefereeError(f"{grammar_path} is not the pinned googlesql.tm")
    rows: list[Row] = refusal_rows(data.decode())
    document: Row = {
        "format": PARSER_REFUSALS_FORMAT,
        "source": PARSER_GRAMMAR,
        "source_sha256": PARSER_GRAMMAR_SHA256,
        "read": PARSER_REFUSALS_READ,
        "refusals": rows,
    }
    PARSER_REFUSALS_FILE.write_text(json.dumps(document, indent=1, ensure_ascii=False) + "\n")
    return len(rows)


@functools.cache
def syntax_classes() -> tuple[ErrorClass, ...]:
    """The committed refusal classes, the parser's own token refusals, the generic class."""
    document: Row = json.loads(PARSER_REFUSALS_FILE.read_text())
    if (document["format"], document["source_sha256"]) != (
        PARSER_REFUSALS_FORMAT,
        PARSER_GRAMMAR_SHA256,
    ):
        raise RefereeError(f"{PARSER_REFUSALS_FILE.name} is not read from the pinned grammar")
    refusals: list[Row] = rows_of(document["refusals"])
    found: list[ErrorClass] = [
        ErrorClass(PARSER_REASON, str(row["pattern"]), str(row["code"]), SYNTAX_ERROR)
        for row in refusals
    ]
    return (*found, *PARSER_INTERNAL_CLASSES, GENERIC_SYNTAX)
