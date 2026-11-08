"""Strict containment: a parse.y sequence is covered only as a contiguous catalogued run."""

from __future__ import annotations

import re

from scripts.syntax_catalogue_sqlite.classes.automaton import Automaton
from scripts.syntax_catalogue_sqlite.constants import (
    GROUP_CLOSE_TOKENS,
    GROUP_CLOSERS,
    INLINE_DEPTH,
    INLINE_EXCLUDED,
    LINE_PUNCTUATION,
    LINE_TOKEN,
    OPTIONAL_OPEN,
    PLACEHOLDER_NAME,
    REPEAT,
    TOKEN_ALTERNATIVE,
)


class _Builder:
    """Thompson construction of one line, with diagrams inlined as optional parts."""

    def __init__(
        self, *, automaton: Automaton, keywords: frozenset[str], definitions: dict[str, str]
    ) -> None:
        self.automaton: Automaton = automaton
        self.keywords: frozenset[str] = keywords
        self.definitions: dict[str, str] = definitions

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

    def word(self, *, token: str, depth: int) -> tuple[int, int]:
        """One token: a keyword or punctuation symbol, a diagram, or nothing."""
        upper: str = token.upper()
        if upper in self.keywords:
            return self.symbol(upper)
        if token in LINE_PUNCTUATION:
            return self.symbol(token)
        match: re.Match[str] | None = PLACEHOLDER_NAME.match(token)
        if match and depth < INLINE_DEPTH and match.group(1) in self.definitions:
            name: str = match.group(1)
            if name not in INLINE_EXCLUDED:
                inner: tuple[int, int] = self.line(
                    tokens=LINE_TOKEN.findall(self.definitions[name]), depth=depth + 1
                )
                return self.choice(parts=[inner], optional=True)
        return self.empty()

    def line(self, *, tokens: list[str], depth: int) -> tuple[int, int]:
        fragment, _ = self.alternatives(tokens=tokens, index=0, closer="", depth=depth)
        return fragment

    def alternatives(
        self, *, tokens: list[str], index: int, closer: str, depth: int
    ) -> tuple[tuple[int, int], int]:
        """`a | b | ...` up to `closer`, and the index after the closer."""
        parts: list[tuple[int, int]] = []
        while True:
            part, index = self.sequence(tokens=tokens, index=index, closer=closer, depth=depth)
            parts.append(part)
            if index < len(tokens) and tokens[index] == TOKEN_ALTERNATIVE:
                index += 1
                continue
            return (
                parts[0] if len(parts) == 1 else self.choice(parts=parts, optional=False)
            ), index + 1

    def sequence(
        self, *, tokens: list[str], index: int, closer: str, depth: int
    ) -> tuple[tuple[int, int], int]:
        """Items up to a bar or `closer`."""
        items: list[tuple[int, int]] = []
        while index < len(tokens) and tokens[index] not in (TOKEN_ALTERNATIVE, closer):
            token: str = tokens[index]
            if token in GROUP_CLOSERS:
                inner, index = self.alternatives(
                    tokens=tokens, index=index + 1, closer=GROUP_CLOSERS[token], depth=depth
                )
                item: tuple[int, int] = (
                    self.choice(parts=[inner], optional=True) if token == OPTIONAL_OPEN else inner
                )
            elif token in GROUP_CLOSE_TOKENS:
                index += 1
                continue
            elif token == REPEAT:
                if items:
                    start, end = items[-1]
                    self.automaton.empty[end].append(start)
                index += 1
                continue
            else:
                item = self.word(token=token, depth=depth)
                index += 1
            items.append(item)
        return self.chain(items), index


def line_automaton(
    *, line: str, keywords: frozenset[str], definitions: dict[str, str]
) -> tuple[Automaton, int]:
    """The automaton of one catalogued line and its start state."""
    automaton: Automaton = Automaton()
    builder: _Builder = _Builder(automaton=automaton, keywords=keywords, definitions=definitions)
    start, _ = builder.line(tokens=LINE_TOKEN.findall(line), depth=0)
    return automaton, start
