"""Parse a normalized catalogue production into a tree of tokens, options and choices."""

from __future__ import annotations

import re

from scripts.grammar_obligations.constants import (
    ASSIGNMENT_SIZE,
    ASSIGNMENTS,
    BAR,
    CLASS_RULES,
    CLOSERS,
    DEFAULT_CLASS,
    GRAMMAR_DEFINITION,
    NAMED_ARGUMENT,
    NODE_CHOICE,
    NODE_OPTIONAL,
    NODE_TOKEN,
    OPEN_CHOICE,
    OPEN_OPTIONAL,
    QUOTED_PLACEHOLDER,
    TOKEN,
    TOKEN_BRACKET,
    TOKEN_ELISION,
    TOKEN_KEYWORD,
    TOKEN_LITERAL,
    TOKEN_PLACEHOLDER,
    TOKEN_PUNCT,
    VALUE_TOKENS,
)
from scripts.grammar_obligations.models import Node


def tokens(production: str) -> list[tuple[str, str]]:
    """(kind, text) pairs of a production; a definition's `<name> ::=` head is dropped."""
    text: str = GRAMMAR_DEFINITION.sub("", production)
    pairs: list[tuple[str, str]] = [
        (match.lastgroup, match.group(match.lastgroup))
        for match in TOKEN.finditer(text)
        if match.lastgroup is not None
    ]
    found: list[tuple[str, str]] = []
    for index, (kind, word) in enumerate(pairs):
        named: bool = index + 1 < len(pairs) and pairs[index + 1][1] == NAMED_ARGUMENT
        bare: bool = kind == TOKEN_KEYWORD and not named and any(char.islower() for char in word)
        found.append((TOKEN_PLACEHOLDER if bare else kind, word))
    return found


def parse(production: str) -> tuple[Node, ...]:
    """The top-level sequence of a production. Unbalanced brackets are kept as plain tokens."""
    stream: list[tuple[str, str]] = tokens(production)
    sequence, _ = _sequence(stream=stream, start=0, closer="")
    return sequence


def _sequence(
    *, stream: list[tuple[str, str]], start: int, closer: str
) -> tuple[tuple[Node, ...], int]:
    """Branches split at top-level bars, up to `closer`; returns them flattened when only one."""
    branches: list[tuple[Node, ...]] = []
    current: list[Node] = []
    index: int = start
    while index < len(stream):
        kind, text = stream[index]
        if kind == TOKEN_BRACKET and text == closer:
            break
        if kind == TOKEN_BRACKET and text == BAR and _assigned_value(current):
            values, index = _bound_values(stream=stream, start=index, first=current[-1])
            if len(values) > 1:
                current[-1] = Node(NODE_CHOICE, OPEN_CHOICE, tuple((value,) for value in values))
                continue
        if kind == TOKEN_BRACKET and text == BAR:
            branches.append(tuple(current))
            current = []
            index += 1
            continue
        if kind == TOKEN_BRACKET and text in CLOSERS:
            inner, end = _group(stream=stream, start=index + 1, closer=CLOSERS[text])
            if end < len(stream):
                current.append(
                    Node(NODE_OPTIONAL if text == OPEN_OPTIONAL else NODE_CHOICE, text, inner)
                )
                index = end + 1
                continue
        current.append(
            Node(NODE_TOKEN, text, token_kind=TOKEN_PUNCT if kind == TOKEN_BRACKET else kind)
        )
        index += 1
    branches.append(tuple(current))
    if len(branches) == 1:
        return branches[0], index
    return (Node(NODE_CHOICE, BAR, tuple(branches)),), index


def _assigned_value(current: list[Node]) -> bool:
    """Whether `current` ends with `KEY = value` or `KEY => value`, a keyword or literal value."""
    return (
        len(current) >= ASSIGNMENT_SIZE
        and current[-2].kind == NODE_TOKEN
        and current[-2].text in ASSIGNMENTS
        and current[-1].kind == NODE_TOKEN
        and current[-1].token_kind in VALUE_TOKENS
    )


def _bound_values(
    *, stream: list[tuple[str, str]], start: int, first: Node
) -> tuple[list[Node], int]:
    """An unbraced `= a | b | c` value list bound to its key, and the index after it."""
    values: list[Node] = [first]
    index: int = start
    while (
        index + 1 < len(stream)
        and stream[index] == (TOKEN_BRACKET, BAR)
        and stream[index + 1][0] in VALUE_TOKENS
        and (index + 2 >= len(stream) or stream[index + 2][1] not in ASSIGNMENTS)
    ):
        values.append(Node(NODE_TOKEN, stream[index + 1][1], token_kind=stream[index + 1][0]))
        index += 2
    return values, index


def _group(
    *, stream: list[tuple[str, str]], start: int, closer: str
) -> tuple[tuple[tuple[Node, ...], ...], int]:
    """The branches inside a bracket pair, and the index of its closer (len(stream) if none)."""
    sequence, end = _sequence(stream=stream, start=start, closer=closer)
    if len(sequence) == 1 and sequence[0].kind == NODE_CHOICE and sequence[0].text == BAR:
        return sequence[0].branches, end
    return (sequence,), end


def render(nodes: tuple[Node, ...]) -> str:
    """Canonical text of a node sequence, with single spaces."""
    return " ".join(_render(node) for node in nodes)


def _render(node: Node) -> str:
    if node.kind == NODE_TOKEN:
        return node.text
    inner: str = " | ".join(render(branch) for branch in node.branches)
    if node.kind == NODE_OPTIONAL:
        return f"[ {inner} ]"
    return f"{{ {inner} }}"


def is_repetition(branch: tuple[Node, ...]) -> bool:
    """A list continuation such as `[ , <x> ... ]`: an elision and no keyword."""
    kinds: set[str] = {node.token_kind for node in branch if node.kind == NODE_TOKEN}
    return TOKEN_ELISION in kinds and TOKEN_KEYWORD not in kinds


def placeholder_name(node: Node) -> str:
    """`<name>` without its brackets, lower-cased; empty for anything else."""
    if node.kind != NODE_TOKEN:
        return ""
    if node.token_kind == TOKEN_PLACEHOLDER:
        return node.text.strip("<>").strip().lower()
    quoted: re.Match[str] | None = QUOTED_PLACEHOLDER.fullmatch(node.text)
    if node.token_kind == TOKEN_LITERAL and quoted:
        return quoted.group(1).lower()
    return ""


def operand_class(name: str) -> str:
    """The operand class of a placeholder name, by the first matching rule."""
    for pattern, label in CLASS_RULES:
        if pattern.search(name):
            return label
    return DEFAULT_CLASS
