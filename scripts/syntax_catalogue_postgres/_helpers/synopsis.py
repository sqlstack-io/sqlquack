"""Split a command synopsis into statements and `where X is:` sub-grammar definitions."""

from __future__ import annotations

import re
from collections.abc import Callable

from scripts.grammar_catalogue._helpers.grammar import (
    bare_words,
    bracket_depth_change,
    normalize,
    required_skeleton,
)
from scripts.syntax_catalogue_postgres.constants import ALTERNATIVE_VERBS, PHRASE_LINE, WHERE_PHRASE
from scripts.syntax_catalogue_postgres.models import Definition, Segment


def indent(line: str) -> int:
    """The number of leading spaces of a line."""
    return len(line) - len(line.lstrip(" "))


def segments(text: str) -> list[Segment]:
    """The block cut at `<phrase>` lines: statements first, then each phrase's body."""
    found: list[Segment] = [Segment(names=(), verb="", lines=[])]
    for raw in text.expandtabs().splitlines():
        phrase: re.Match[str] | None = PHRASE_LINE.match(raw)
        if phrase is None:
            found[-1].lines.append(raw.rstrip())
            continue
        words: str = normalize(phrase.group(1))
        where: re.Match[str] | None = WHERE_PHRASE.match(words)
        if where is None:
            found.append(Segment(names=(), verb=words, lines=[]))
            continue
        names: tuple[str, ...] = tuple(
            name.strip().strip("<>") for name in re.split(r",|\band\b", where.group("names"))
        )
        found.append(
            Segment(names=tuple(n for n in names if n), verb=where.group("verb").lower(), lines=[])
        )
    return found


def split_at(*, lines: list[str], starts: Callable[[str, str], bool]) -> list[str]:
    """Join lines into productions, starting a new one where `starts(line, current)` says so."""
    out: list[list[str]] = []
    depth: int = 0
    for line in lines:
        if not line.strip():
            continue
        current: str = normalize(" ".join(out[-1])) if out else ""
        if not out or (depth <= 0 and starts(line, current)):
            out.append([line])
            depth = bracket_depth_change(line)
            continue
        out[-1].append(line)
        depth += bracket_depth_change(line)
    return [normalize(" ".join(chunk)) for chunk in out]


def statement_start(*, line: str, current: str, leads: frozenset[str]) -> bool:
    """True when an unindented line with a lead word follows a statement with keywords."""
    words: list[str] = bare_words(line)
    return (
        indent(line) == 0
        and bool(words)
        and words[0] in leads
        and bool(bare_words(required_skeleton(current)))
    )


def statements(*, lines: list[str], leads: frozenset[str]) -> list[str]:
    """Statements of a synopsis segment: each starts at an unindented line with a lead word."""
    return split_at(
        lines=lines,
        starts=lambda line, current: statement_start(line=line, current=current, leads=leads),
    )


def alternatives(lines: list[str]) -> list[str]:
    """The alternatives of a `can be one of:` list: one per line at the list's own indent."""
    body: list[str] = [line for line in lines if line.strip()]
    if not body:
        return []
    base: int = min(indent(line) for line in body)
    parts: list[str] = split_at(lines=body, starts=lambda line, _current: indent(line) == base)
    return [part.removeprefix("| ").removesuffix(" |").strip() for part in parts]


def read_synopsis(
    *, text: str, statement_leads: frozenset[str]
) -> tuple[list[str], list[Definition], bool]:
    """The block's statements, its definitions, and whether a phrase restarted statements."""
    found: list[str] = []
    definitions: list[Definition] = []
    restarted: bool = False
    for segment in segments(text):
        if not segment.names:
            restarted = restarted or bool(segment.verb)
            found += statements(lines=segment.lines, leads=statement_leads)
            continue
        body, tail = definition_body(lines=segment.lines, statement_leads=statement_leads)
        found += statements(lines=tail, leads=statement_leads)
        listed: bool = segment.verb in ALTERNATIVE_VERBS
        parts: list[str] = alternatives(body) if listed else [normalize(" ".join(body))]
        for name in segment.names:
            definitions.append(
                Definition(name=name, alternatives=tuple(p for p in parts if p), listed=listed)
            )
    return found, definitions, restarted


def definition_body(
    *, lines: list[str], statement_leads: frozenset[str]
) -> tuple[list[str], list[str]]:
    """A definition's lines, and any statement that follows it unindented (SELECT's TABLE)."""
    for index, line in enumerate(lines):
        words: list[str] = bare_words(line)
        if index and indent(line) == 0 and words and words[0] in statement_leads:
            return lines[:index], lines[index:]
    return lines, []


def definition_grammar(definition: Definition) -> str:
    """`<name> ::= ...`, with listed alternatives as one braced choice."""
    if definition.listed and len(definition.alternatives) > 1:
        return f"<{definition.name}> ::= {{ " + " | ".join(definition.alternatives) + " }"
    return f"<{definition.name}> ::= " + " ".join(definition.alternatives)
