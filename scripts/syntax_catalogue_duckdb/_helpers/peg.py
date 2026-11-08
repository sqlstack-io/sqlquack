"""Read DuckDB's PEG grammar files (`statements/*.gram`) into rules of expression trees."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.syntax_catalogue_duckdb.constants import (
    NODE_AND,
    NODE_ANY,
    NODE_CALL,
    NODE_CAPTURE,
    NODE_CHOICE,
    NODE_CLASS,
    NODE_LIT,
    NODE_NOT,
    NODE_REF,
    NODE_SEQ,
    PEG_ALTERNATIVE,
    PEG_ANY,
    PEG_ARGUMENT_SEPARATOR,
    PEG_CAPTURE_CLOSE,
    PEG_CAPTURE_OPEN,
    PEG_CLASS_CLOSE,
    PEG_CLASS_OPEN,
    PEG_COMMENT,
    PEG_ESCAPE,
    PEG_GROUP_CLOSE,
    PEG_GROUP_OPEN,
    PEG_NAME_START,
    PEG_NOT,
    PEG_PREFIXES,
    PEG_QUOTE_OPENERS,
    PEG_RULE_HEAD,
    PEG_SEQUENCE_ENDS,
    PEG_SUFFIXES,
    PEG_TOKEN,
)
from scripts.syntax_catalogue_duckdb.exceptions import DuckDbCatalogueError
from scripts.syntax_catalogue_duckdb.models import PegRule
from scripts.syntax_catalogue_duckdb.types import Node


def rule_texts(text: str) -> list[tuple[str, tuple[str, ...], str]]:
    """Each rule as (name, macro parameters, body), comments removed and lines joined."""
    found: list[tuple[str, tuple[str, ...], list[str]]] = []
    for line in text.splitlines():
        code: str = strip_comment(line)
        head: re.Match[str] | None = PEG_RULE_HEAD.match(code)
        if head:
            parameters: tuple[str, ...] = tuple(
                part.strip() for part in (head.group(2) or "").split(",") if part.strip()
            )
            found.append((head.group(1), parameters, [code[head.end() :]]))
        elif code.strip() and found:
            found[-1][2].append(code)
    return [(name, parameters, " ".join(body)) for name, parameters, body in found]


def strip_comment(line: str) -> str:
    """The line up to a `#` outside quotes and character classes."""
    quote: str = ""
    for index, ch in enumerate(line):
        if quote:
            if ch == PEG_ESCAPE:
                continue
            if ch == quote and line[index - 1] != PEG_ESCAPE:
                quote = ""
        elif ch in PEG_QUOTE_OPENERS:
            quote = PEG_CLASS_CLOSE if ch == PEG_CLASS_OPEN else ch
        elif ch == PEG_COMMENT:
            return line[:index]
    return line


def tokens(body: str) -> list[str]:
    """The body's tokens: literals, classes, names, macro calls and operators."""
    out: list[str] = []
    position: int = 0
    while position < len(body):
        if body[position].isspace():
            position += 1
            continue
        match: re.Match[str] | None = PEG_TOKEN.match(body, position)
        if match is None:
            raise DuckDbCatalogueError(f"unreadable grammar text at: {body[position:][:40]!r}")
        out.append(match.group(0))
        position = match.end()
    return out


class _Reader:
    """A recursive-descent reader of one rule body."""

    def __init__(self, items: list[str]) -> None:
        self.items: list[str] = items
        self.index: int = 0

    def peek(self) -> str:
        return self.items[self.index] if self.index < len(self.items) else ""

    def take(self) -> str:
        item: str = self.peek()
        self.index += 1
        return item

    def choice(self) -> Node:
        options: list[Node] = [self.sequence()]
        while self.peek() == PEG_ALTERNATIVE:
            self.take()
            options.append(self.sequence())
        return options[0] if len(options) == 1 else (NODE_CHOICE, options)

    def sequence(self) -> Node:
        parts: list[Node] = []
        while self.peek() not in PEG_SEQUENCE_ENDS:
            parts.append(self.prefixed())
        return parts[0] if len(parts) == 1 else (NODE_SEQ, parts)

    def prefixed(self) -> Node:
        if self.peek() in PEG_PREFIXES:
            operator: str = self.take()
            return (NODE_NOT if operator == PEG_NOT else NODE_AND, self.suffixed())
        return self.suffixed()

    def suffixed(self) -> Node:
        node: Node = self.primary()
        while self.peek() in PEG_SUFFIXES:
            node = (PEG_SUFFIXES[self.take()], node)
        return node

    def primary(self) -> Node:
        item: str = self.take()
        if item == PEG_GROUP_OPEN:
            node: Node = self.choice()
            self.expect(PEG_GROUP_CLOSE)
            return node
        if item == PEG_CAPTURE_OPEN:
            node = self.choice()
            self.expect(PEG_CAPTURE_CLOSE)
            return (NODE_CAPTURE, node)
        if item.endswith(PEG_GROUP_OPEN):
            arguments: list[Node] = [self.choice()]
            while self.peek() == PEG_ARGUMENT_SEPARATOR:
                self.take()
                arguments.append(self.choice())
            self.expect(PEG_GROUP_CLOSE)
            return (NODE_CALL, item[:-1], arguments)
        if item.startswith("'") or item.startswith('"'):
            return (NODE_LIT, item[1:-1].replace("\\'", "'"))
        if item.startswith(PEG_CLASS_OPEN):
            return (NODE_CLASS, item)
        if item == PEG_ANY:
            return (NODE_ANY,)
        if item and (item[0].isalpha() or item[0] == PEG_NAME_START):
            return (NODE_REF, item)
        raise DuckDbCatalogueError(f"unexpected grammar token {item!r}")

    def expect(self, item: str) -> None:
        if self.take() != item:
            raise DuckDbCatalogueError(f"expected {item!r} in grammar")


def read_rules(folder: Path) -> dict[str, PegRule]:
    """Every rule of every `.gram` file in the folder, by name."""
    rules: dict[str, PegRule] = {}
    for path in sorted(folder.glob("*.gram")):
        for name, parameters, body in rule_texts(path.read_text(encoding="utf-8")):
            if name.startswith("%"):
                continue
            reader: _Reader = _Reader(tokens(body))
            node: Node = reader.choice()
            if reader.peek():
                raise DuckDbCatalogueError(f"{path.name}: {name} has trailing text")
            if name in rules:
                raise DuckDbCatalogueError(f"{path.name}: {name} is defined twice")
            rules[name] = PegRule(name=name, parameters=parameters, body=node, file=path.name)
    return rules
