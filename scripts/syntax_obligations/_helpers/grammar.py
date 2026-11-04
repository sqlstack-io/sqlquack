"""Parse a normalized catalogue production into a tree of tokens, options and choices."""

from __future__ import annotations

import re

from scripts.syntax_obligations.constants import (
    BAR,
    CLOSE_CHOICE,
    CLOSE_OPTIONAL,
    NODE_CHOICE,
    NODE_OPTIONAL,
    NODE_TOKEN,
    OPEN_CHOICE,
    OPEN_OPTIONAL,
    TOKEN,
    TOKEN_ELISION,
    TOKEN_KEYWORD,
    TOKEN_LITERAL,
    TOKEN_PLACEHOLDER,
)
from scripts.syntax_obligations.models import Node

DEFINITION: re.Pattern[str] = re.compile(r"^\s*<[^<>]+>\s*::=\s*")
QUOTED_PLACEHOLDER: re.Pattern[str] = re.compile(r"'<([^<>]+)>'")
CLOSERS: dict[str, str] = {OPEN_OPTIONAL: CLOSE_OPTIONAL, OPEN_CHOICE: CLOSE_CHOICE}


def tokens(production: str) -> list[tuple[str, str]]:
    """(kind, text) pairs of a production; a definition's `<name> ::=` head is dropped."""
    text: str = DEFINITION.sub("", production)
    found: list[tuple[str, str]] = []
    for match in TOKEN.finditer(text):
        kind: str | None = match.lastgroup
        if kind is not None:
            word: str = match.group(kind)
            bare: bool = kind == TOKEN_KEYWORD and any(char.islower() for char in word)
            found.append((TOKEN_PLACEHOLDER if bare else kind, word))
    return found


def parse(production: str) -> tuple[Node, ...]:
    """The top-level sequence of a production. Unbalanced brackets are kept as plain tokens."""
    stream: list[tuple[str, str]] = tokens(production)
    sequence, _ = _sequence(stream, 0, "")
    return sequence


def _sequence(
    stream: list[tuple[str, str]], start: int, closer: str
) -> tuple[tuple[Node, ...], int]:
    """Branches split at top-level bars, up to `closer`; returns them flattened when only one."""
    branches: list[tuple[Node, ...]] = []
    current: list[Node] = []
    index: int = start
    while index < len(stream):
        kind, text = stream[index]
        if kind == "bracket" and text == closer:
            break
        if kind == "bracket" and text == BAR:
            branches.append(tuple(current))
            current = []
            index += 1
            continue
        if kind == "bracket" and text in CLOSERS:
            inner, end = _group(stream, index + 1, CLOSERS[text])
            if end < len(stream):
                current.append(
                    Node(NODE_OPTIONAL if text == OPEN_OPTIONAL else NODE_CHOICE, text, inner)
                )
                index = end + 1
                continue
        current.append(Node(NODE_TOKEN, text, token_kind="punct" if kind == "bracket" else kind))
        index += 1
    branches.append(tuple(current))
    if len(branches) == 1:
        return branches[0], index
    return (Node(NODE_CHOICE, BAR, tuple(branches)),), index


def _group(
    stream: list[tuple[str, str]], start: int, closer: str
) -> tuple[tuple[tuple[Node, ...], ...], int]:
    """The branches inside a bracket pair, and the index of its closer (len(stream) if none)."""
    sequence, end = _sequence(stream, start, closer)
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
    return TOKEN_ELISION in kinds and "keyword" not in kinds


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
