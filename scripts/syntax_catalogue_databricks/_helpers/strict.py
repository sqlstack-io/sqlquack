"""Strict containment: a grammar sequence is covered only as a contiguous run of a line."""

from __future__ import annotations

import re

from scripts.syntax_catalogue_databricks.classes.automaton import Automaton
from scripts.syntax_catalogue_databricks.constants import (
    BRACKET_ENDS,
    CHOICE_BAR,
    CLAUSE_SUFFIX,
    CLOSERS,
    ESCAPED_PIPE,
    INLINE_DEPTH,
    KEYWORD_SYMBOL,
    LINE_TOKEN,
    OPTIONAL_OPEN,
    PLACEHOLDER_NAME,
    PUNCTUATION,
    REPEAT,
    SPELLING_SYNONYMS,
)


class _Builder:
    """Thompson construction of one line, with definitions inlined as optional parts."""

    def __init__(
        self,
        *,
        automaton: Automaton,
        own: dict[str, str],
        definitions: dict[str, str],
        clauses: dict[tuple[str, ...], list[list[str]]],
    ) -> None:
        self.automaton: Automaton = automaton
        self.own: dict[str, str] = own
        self.definitions: dict[str, str] = definitions
        self.clauses: dict[tuple[str, ...], list[list[str]]] = clauses

    def empty(self) -> tuple[int, int]:
        start: int = self.automaton.state()
        end: int = self.automaton.state()
        self.automaton.empty[start].append(end)
        return start, end

    def symbol(self, label: str) -> tuple[int, int]:
        start: int = self.automaton.state()
        end: int = self.automaton.state()
        self.automaton.edges[start].append((label, end))
        return start, end

    def chain(self, parts: list[tuple[int, int]]) -> tuple[int, int]:
        if not parts:
            return self.empty()
        for (_, end), (start, _) in zip(parts, parts[1:], strict=False):
            self.automaton.empty[end].append(start)
        return parts[0][0], parts[-1][1]

    def choice(self, *, parts: list[tuple[int, int]], optional: bool) -> tuple[int, int]:
        start: int = self.automaton.state()
        end: int = self.automaton.state()
        for part_start, part_end in parts:
            self.automaton.empty[start].append(part_start)
            self.automaton.empty[part_end].append(end)
        if optional:
            self.automaton.empty[start].append(end)
        return start, end

    def clause(self, *, recent: list[str], depth: int) -> tuple[int, int] | None:
        """The forms a clause placeholder after `recent` keywords names, their shared lead cut."""
        for size in range(len(recent), 0, -1):
            found: list[list[str]] | None = self.clauses.get(tuple(recent[-size:]))
            if found:
                inner: list[tuple[int, int]] = [
                    self.alternatives(tokens=rest, index=0, closer="", depth=depth + 1)[0]
                    for rest in found
                ]
                return self.choice(parts=inner, optional=True)
        return None

    def word(self, *, token: str, depth: int, recent: list[str]) -> tuple[int, int]:
        """One token: a word or compared punctuation, a definition, a clause, or nothing."""
        if KEYWORD_SYMBOL.match(token):
            return self.symbol(token)
        if token == ESCAPED_PIPE:
            return self.symbol(CHOICE_BAR)
        if token in SPELLING_SYNONYMS:
            return self.symbol(SPELLING_SYNONYMS[token])
        if token in PUNCTUATION:
            return self.symbol(token)
        match: re.Match[str] | None = PLACEHOLDER_NAME.match(token)
        if match and depth < INLINE_DEPTH:
            body: str | None = self.own.get(match.group(1), self.definitions.get(match.group(1)))
            if body is not None:
                inner: tuple[int, int] = self.line(tokens=LINE_TOKEN.findall(body), depth=depth + 1)
                return self.choice(parts=[inner], optional=True)
            if match.group(1).endswith(CLAUSE_SUFFIX) and recent:
                linked: tuple[int, int] | None = self.clause(recent=recent, depth=depth)
                if linked is not None:
                    return linked
        return self.empty()

    def line(self, *, tokens: list[str], depth: int) -> tuple[int, int]:
        fragment, _ = self.alternatives(tokens=tokens, index=0, closer="", depth=depth)
        return fragment

    def alternatives(
        self, *, tokens: list[str], index: int, closer: str, depth: int
    ) -> tuple[tuple[int, int], int]:
        parts: list[tuple[int, int]] = []
        while True:
            part, index = self.sequence(tokens=tokens, index=index, closer=closer, depth=depth)
            parts.append(part)
            if index < len(tokens) and tokens[index] == CHOICE_BAR:
                index += 1
                continue
            return (
                parts[0] if len(parts) == 1 else self.choice(parts=parts, optional=False)
            ), index + 1

    def sequence(
        self, *, tokens: list[str], index: int, closer: str, depth: int
    ) -> tuple[tuple[int, int], int]:
        items: list[tuple[int, int]] = []
        recent: list[str] = []
        while index < len(tokens) and tokens[index] not in (CHOICE_BAR, closer):
            token: str = tokens[index]
            before: list[str] = recent
            recent = [*recent, token] if KEYWORD_SYMBOL.match(token) else []
            if token == OPTIONAL_OPEN and tokens[index + 1 : index + 3] == [
                REPEAT,
                BRACKET_ENDS[0],
            ]:
                if items:
                    start, end = items[-1]
                    self.automaton.empty[end].append(start)
                index += 3
                continue
            if (
                token == OPTIONAL_OPEN
                and tokens[index + 2 : index + 4] == [REPEAT, BRACKET_ENDS[0]]
                and tokens[index + 1] not in (*CLOSERS, *BRACKET_ENDS, CHOICE_BAR)
            ):
                if items:
                    start, end = items[-1]
                    separator: tuple[int, int] = self.word(
                        token=tokens[index + 1], depth=depth, recent=[]
                    )
                    self.automaton.empty[end].append(separator[0])
                    self.automaton.empty[separator[1]].append(start)
                index += 4
                continue
            if token in CLOSERS:
                inner, index = self.alternatives(
                    tokens=tokens, index=index + 1, closer=CLOSERS[token], depth=depth
                )
                item: tuple[int, int] = (
                    self.choice(parts=[inner], optional=True) if token == OPTIONAL_OPEN else inner
                )
            elif token in BRACKET_ENDS:
                index += 1
                continue
            elif token == REPEAT:
                if items:
                    start, end = items[-1]
                    self.automaton.empty[end].append(start)
                index += 1
                continue
            else:
                item = self.word(token=token, depth=depth, recent=before)
                index += 1
            items.append(item)
        return self.chain(items), index


def clause_index(lines: list[str]) -> dict[tuple[str, ...], list[list[str]]]:
    """Each leading keyword run of the lines, and every prefix of it, to the tokens after it."""
    index: dict[tuple[str, ...], list[list[str]]] = {}
    for line in lines:
        tokens: list[str] = LINE_TOKEN.findall(line)
        lead: int = 0
        while lead < len(tokens) and KEYWORD_SYMBOL.match(tokens[lead]):
            lead += 1
        for size in range(1, lead + 1):
            index.setdefault(tuple(tokens[:size]), []).append(tokens[size:])
    return index


def line_automaton(
    *,
    line: str,
    own: dict[str, str],
    definitions: dict[str, str],
    clauses: dict[tuple[str, ...], list[list[str]]],
) -> tuple[Automaton, int]:
    """The automaton of one catalogued line and its start state."""
    automaton: Automaton = Automaton()
    builder: _Builder = _Builder(
        automaton=automaton, own=own, definitions=definitions, clauses=clauses
    )
    start, _ = builder.line(tokens=LINE_TOKEN.findall(line), depth=0)
    return automaton, start
