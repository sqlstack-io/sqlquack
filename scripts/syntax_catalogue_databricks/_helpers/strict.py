"""Strict containment: a grammar sequence is covered only as a contiguous run of a catalogued line.

Each catalogued grammar line becomes an automaton over its upper-case words and the punctuation
the audit compares. Placeholders, quoted text, numbers and escaped notation characters spell
nothing, exactly as identifiers, literals and the other tokens spell nothing in a grammar
sequence. A placeholder that a definition of the line's page (or, failing that, of any page)
names stands for that definition or for nothing, as a rule too large to spell stands for nothing.
`[ ]` is optional, `{ a | b }` a choice and `...` repeats the item it follows. A sequence is
covered when the automaton reads it without a gap from some state; a statement's from the
line's start.
"""

from __future__ import annotations

import re

from scripts.syntax_catalogue_databricks.constants import (
    ANY_RUN,
    CLAUSE_SUFFIX,
    INLINE_DEPTH,
    KEYWORD_SYMBOL,
    PUNCTUATION_TOKENS,
    SPELLING_SYNONYMS,
)

LINE_TOKEN: re.Pattern[str] = re.compile(
    r"\\[|\[\]{}]|<[A-Za-z_][^<>]*>|\.\.\.|\"[^\"]*\"|'[^']*'|<=>|>>>|>>|<<|==|<=|>=|<>|!=|!>|!<"
    r"|\|\||\|>|::|->|=>|[\[\]{}|]|[A-Za-z_][A-Za-z0-9_]*|\S"
)
PUNCTUATION: frozenset[str] = frozenset(
    spelling for spellings in PUNCTUATION_TOKENS.values() for spelling in spellings
)
ESCAPED_PIPE: str = "\\|"
PLACEHOLDER_NAME: re.Pattern[str] = re.compile(r"^<([^<>\s][^<>]*)>$")
REPEAT: str = "..."
CLOSERS: dict[str, str] = {"[": "]", "{": "}"}


class Automaton:
    """A nondeterministic automaton: labelled and empty edges between numbered states."""

    def __init__(self) -> None:
        self.edges: list[list[tuple[str, int]]] = []
        self.empty: list[list[int]] = []

    def state(self) -> int:
        self.edges.append([])
        self.empty.append([])
        return len(self.edges) - 1

    def closure(self, states: set[int]) -> set[int]:
        found: set[int] = set(states)
        frontier: list[int] = list(states)
        while frontier:
            for target in self.empty[frontier.pop()]:
                if target not in found:
                    found.add(target)
                    frontier.append(target)
        return found

    def reach(self, states: set[int]) -> set[int]:
        """The states reachable along any edges: what a rule too large to spell may read."""
        found: set[int] = set(states)
        frontier: list[int] = list(states)
        while frontier:
            state: int = frontier.pop()
            for target in [*self.empty[state], *(target for _, target in self.edges[state])]:
                if target not in found:
                    found.add(target)
                    frontier.append(target)
        return found

    def targets(self, label: str) -> set[int]:
        """Every state an edge labelled `label` leads to, from any state."""
        if not hasattr(self, "_targets"):
            index: dict[str, set[int]] = {}
            for edges in self.edges:
                for found, target in edges:
                    index.setdefault(found, set()).add(target)
            self._targets: dict[str, set[int]] = index
        return self._targets.get(label, set())

    def reads(self, *, sequence: tuple[str, ...], start: int | None) -> bool:
        """True when the sequence is read without a gap from `start`, or from any state."""
        current: set[int] | None = None if start is None else self.closure({start})
        for symbol in sequence:
            if symbol == ANY_RUN:
                if current is not None:
                    current = self.reach(current)
                continue
            moved: set[int] = (
                self.targets(symbol)
                if current is None
                else {
                    target
                    for state in current
                    for label, target in self.edges[state]
                    if label == symbol
                }
            )
            if not moved:
                return False
            current = self.closure(moved)
        return True


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

    def choice(self, parts: list[tuple[int, int]], *, optional: bool) -> tuple[int, int]:
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
                return self.choice(inner, optional=True)
        return None

    def word(self, token: str, depth: int, recent: list[str]) -> tuple[int, int]:
        """One token: a word or compared punctuation, a definition, a clause, or nothing."""
        if KEYWORD_SYMBOL.match(token):
            return self.symbol(token)
        if token == ESCAPED_PIPE:
            return self.symbol("|")
        if token in SPELLING_SYNONYMS:
            return self.symbol(SPELLING_SYNONYMS[token])
        if token in PUNCTUATION:
            return self.symbol(token)
        match: re.Match[str] | None = PLACEHOLDER_NAME.match(token)
        if match and depth < INLINE_DEPTH:
            body: str | None = self.own.get(match.group(1), self.definitions.get(match.group(1)))
            if body is not None:
                inner: tuple[int, int] = self.line(tokens=LINE_TOKEN.findall(body), depth=depth + 1)
                return self.choice([inner], optional=True)
            if match.group(1).endswith(CLAUSE_SUFFIX) and recent:  # a clause page's form
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
            if index < len(tokens) and tokens[index] == "|":
                index += 1
                continue
            return (parts[0] if len(parts) == 1 else self.choice(parts, optional=False)), index + 1

    def sequence(
        self, *, tokens: list[str], index: int, closer: str, depth: int
    ) -> tuple[tuple[int, int], int]:
        items: list[tuple[int, int]] = []
        recent: list[str] = []
        while index < len(tokens) and tokens[index] not in ("|", closer):
            token: str = tokens[index]
            before: list[str] = recent
            recent = [*recent, token] if KEYWORD_SYMBOL.match(token) else []
            if token == "[" and tokens[index + 1 : index + 3] == [REPEAT, "]"]:
                if items:
                    start, end = items[-1]
                    self.automaton.empty[end].append(start)
                index += 3
                continue
            if (
                token == "["
                and tokens[index + 2 : index + 4] == [REPEAT, "]"]
                and tokens[index + 1] not in (*CLOSERS, "]", "}", "|")
            ):  # `x [ , ... ]`: x, then the separator and x again any number of times
                if items:
                    start, end = items[-1]
                    separator: tuple[int, int] = self.word(tokens[index + 1], depth, [])
                    self.automaton.empty[end].append(separator[0])
                    self.automaton.empty[separator[1]].append(start)
                index += 4
                continue
            if token in CLOSERS:
                inner, index = self.alternatives(
                    tokens=tokens, index=index + 1, closer=CLOSERS[token], depth=depth
                )
                item: tuple[int, int] = (
                    self.choice([inner], optional=True) if token == "[" else inner
                )
            elif token in ("]", "}"):
                index += 1
                continue
            elif token == REPEAT:
                if items:
                    start, end = items[-1]
                    self.automaton.empty[end].append(start)
                index += 1
                continue
            else:
                item = self.word(token, depth, before)
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
