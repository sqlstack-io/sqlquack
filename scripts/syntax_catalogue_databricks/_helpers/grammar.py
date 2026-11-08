"""Read the pinned ANTLR 4 grammar SqlBaseParser.g4 and the keyword tokens of SqlBaseLexer.g4."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.syntax_catalogue_databricks.constants import (
    G4_LABEL_ASSIGN,
    G4_RULE_HEAD,
    KEYWORD_LIST,
    LEXER_TOKEN,
    LEXER_WORD,
    NODE_CHOICE,
    NODE_EMPTY,
    NODE_OPTIONAL,
    NODE_PLUS,
    NODE_REF,
    NODE_SEQ,
    NODE_STAR,
    NODE_TOKEN,
    PARSER_FILE,
    SPARK_DIR,
)
from scripts.syntax_catalogue_databricks.exceptions import DatabricksCatalogueError
from scripts.syntax_catalogue_databricks.types import Node

G4_LEXEME: re.Pattern[str] = re.compile(
    r"(?P<word>[A-Za-z_][A-Za-z0-9_]*)|(?P<quoted>'(?:\\.|[^'\\])*')|(?P<punct>\+=|[:;|()?*+=#~.@])"
)


def strip_comments(text: str) -> str:
    """The grammar without `//` and `/* */` comments; quoted literals kept whole."""
    out: list[str] = []
    index: int = 0
    while index < len(text):
        if text.startswith("//", index):
            end: int = text.find("\n", index)
            index = len(text) if end < 0 else end
            continue
        if text.startswith("/*", index):
            end = text.find("*/", index + 2)
            if end < 0:
                raise DatabricksCatalogueError(f"unterminated comment at offset {index}")
            index = end + 2
            continue
        if text[index] == "'":
            match: re.Match[str] | None = re.compile(r"'(?:\\.|[^'\\])*'").match(text, index)
            if match is None:
                raise DatabricksCatalogueError(f"unterminated literal at offset {index}")
            out.append(match.group(0))
            index = match.end()
            continue
        out.append(text[index])
        index += 1
    return "".join(out)


def balanced_end(*, text: str, start: int) -> int:
    """The index after the `{ }` action opened at `start`."""
    depth: int = 0
    index: int = start
    while index < len(text):
        ch: str = text[index]
        if ch in "\"'":
            close: int = index + 1
            while close < len(text) and text[close] != ch:
                close += 2 if text[close] == "\\" else 1
            index = close + 1
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1
    raise DatabricksCatalogueError(f"unbalanced action at offset {start}")


def lexemes(text: str) -> list[tuple[str, str]]:
    """Words, quoted literals and punctuation; an action is ('action', ''), a predicate empty."""
    out: list[tuple[str, str]] = []
    index: int = 0
    while index < len(text):
        if text[index].isspace():
            index += 1
            continue
        if text[index] == "{":
            end: int = balanced_end(text=text, start=index)
            index = end + 1 if text.startswith("?", end) else end
            out.append(("action", ""))
            continue
        match: re.Match[str] | None = G4_LEXEME.match(text, index)
        if match is None:
            raise DatabricksCatalogueError(f"unexpected {text[index]!r} at offset {index}")
        kind: str = str(match.lastgroup)
        out.append((kind, match.group(0)))
        index = match.end()
    return out


class _Parser:
    """A recursive-descent reader of ANTLR 4 parser rules."""

    def __init__(self, items: list[tuple[str, str]]) -> None:
        self.items: list[tuple[str, str]] = items
        self.index: int = 0

    def peek(self, offset: int = 0) -> tuple[str, str]:
        position: int = self.index + offset
        return self.items[position] if position < len(self.items) else ("", "")

    def take(self) -> tuple[str, str]:
        item: tuple[str, str] = self.peek()
        self.index += 1
        return item

    def expect(self, value: str) -> None:
        found: tuple[str, str] = self.take()
        if found[1] != value:
            raise DatabricksCatalogueError(f"expected {value!r}, found {found[1]!r}")

    def rules(self) -> dict[str, Node]:
        """Every rule after the grammar header, options and member actions."""
        found: dict[str, Node] = {}
        while self.peek()[0]:
            if self.peek() == ("word", "parser"):
                while self.take()[1] != ";":
                    pass
                continue
            if self.peek() in (("word", "options"), ("punct", "@")):
                self.take()
                if self.peek()[0] == "word":
                    self.take()
                if self.take()[0] != "action":
                    raise DatabricksCatalogueError("a header section without its block")
                continue
            if self.peek()[0] == "word" and self.peek(1) == ("punct", ":"):
                name: str = self.take()[1]
                self.take()
                try:
                    body: Node = self.alternatives(";")
                except DatabricksCatalogueError as error:
                    raise DatabricksCatalogueError(f"{PARSER_FILE} rule {name}: {error}") from error
                self.expect(";")
                if name in found:
                    raise DatabricksCatalogueError(f"{PARSER_FILE}: rule {name} defined twice")
                found[name] = body
                continue
            raise DatabricksCatalogueError(f"unexpected {self.peek()[1]!r} between rules")
        return found

    def alternatives(self, closer: str) -> Node:
        branches: list[Node] = [self.sequence(closer)]
        while self.peek() == ("punct", "|"):
            self.take()
            branches.append(self.sequence(closer))
        return branches[0] if len(branches) == 1 else (NODE_CHOICE, tuple(branches))

    def sequence(self, closer: str) -> Node:
        items: list[Node] = []
        while self.peek() not in (("punct", closer), ("punct", "|")):
            if not self.peek()[0]:
                raise DatabricksCatalogueError(f"unexpected end of grammar before {closer!r}")
            if self.peek() == ("punct", "#"):
                self.take()
                self.take()
                continue
            items.append(self.element())
        return (NODE_SEQ, tuple(items))

    def element(self) -> Node:
        """One element with its suffix: an action, subrule, token, rule reference or set."""
        kind, value = self.take()
        if kind == "word" and self.peek()[1] in G4_LABEL_ASSIGN:
            self.take()
            kind, value = self.take()
        node: Node
        if kind == "action":
            node = (NODE_EMPTY,)
        elif (kind, value) == ("punct", "("):
            node = self.alternatives(")")
            self.expect(")")
        elif (kind, value) == ("punct", "~"):
            negated: Node = self.element()
            node = (NODE_TOKEN, f"~{negated[1] if len(negated) > 1 else ''}")
        elif (kind, value) == ("punct", "."):
            node = (NODE_TOKEN, ".")
        elif kind == "quoted":
            node = (NODE_TOKEN, value)
        elif kind == "word":
            node = (NODE_TOKEN, value) if value[:1].isupper() else (NODE_REF, value)
        else:
            raise DatabricksCatalogueError(f"unexpected {value!r} in a rule")
        suffix: str = self.peek()[1] if self.peek()[0] == "punct" else ""
        wrappers: dict[str, str] = {"?": NODE_OPTIONAL, "*": NODE_STAR, "+": NODE_PLUS}
        if suffix in wrappers:
            self.take()
            if self.peek() == ("punct", "?"):
                self.take()
            node = (wrappers[suffix], node)
        return node


def read_rules(cache: Path) -> dict[str, Node]:
    """SqlBaseParser.g4's rules as trees; actions and predicates are empty nodes."""
    text: str = strip_comments((cache / SPARK_DIR / Path(PARSER_FILE).name).read_text())
    rules: dict[str, Node] = _Parser(lexemes(text)).rules()
    heads: int = len(G4_RULE_HEAD.findall(text))
    if heads != len(rules):
        raise DatabricksCatalogueError(f"read {len(rules)} rules; the grammar has {heads} heads")
    return rules


def read_keywords(cache: Path) -> dict[str, tuple[str, ...]]:
    """The keyword tokens between SqlBaseLexer.g4's keyword-list markers: token to words."""
    text: str = (cache / SPARK_DIR / "SqlBaseLexer.g4").read_text()
    block: re.Match[str] | None = KEYWORD_LIST.search(text)
    if block is None:
        raise DatabricksCatalogueError("SqlBaseLexer.g4: no keyword list markers")
    found: dict[str, tuple[str, ...]] = {}
    for name, body in LEXER_TOKEN.findall(block.group(1)):
        words: list[str] = LEXER_WORD.findall(body)
        if not words:
            raise DatabricksCatalogueError(f"SqlBaseLexer.g4: keyword {name} spells no word")
        found[name] = tuple(words)
    return found
