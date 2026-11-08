"""Read the parser section of GoogleSQL's Textmapper grammar (googlesql.tm) into rule trees."""

from __future__ import annotations

import itertools
import re
from pathlib import Path

from scripts.syntax_catalogue_bigquery.constants import (
    DISPATCH_CALL,
    DISPATCH_PREFIX,
    FEATURE_TEST,
    GRAMMAR_DIR,
    GRAMMAR_FILE,
    HELPER_HEAD,
    NODE_CHOICE,
    NODE_LIST,
    NODE_LIT,
    NODE_OPT,
    NODE_REF,
    NODE_SEQ,
    NODE_SET,
    NODE_TOKEN,
    OPT_SUFFIX,
    REJECTED,
    REJECTING_ACTION,
    SEPARATOR,
    TM_ACTION_CLOSE,
    TM_ACTION_OPEN,
    TM_ALTERNATIVE,
    TM_BLOCK_CLOSE,
    TM_BLOCK_OPEN,
    TM_DIRECTIVE,
    TM_END,
    TM_ESCAPE,
    TM_GROUP_CLOSE,
    TM_GROUP_OPEN,
    TM_HASH_COMMENT,
    TM_LABEL_CLOSE,
    TM_LABEL_OPEN,
    TM_LINE_COMMENT,
    TM_MARKER,
    TM_NAME,
    TM_PARSER,
    TM_PREC,
    TM_QUOTES,
    TM_RULE_COLON,
    TM_RULE_END,
    TM_STRING_QUOTE,
    TM_TOKEN_NAME,
    TM_WRAPPERS,
    TOKEN_ACTION,
    TOKEN_DIRECTIVE,
    TOKEN_END,
    TOKEN_NAME,
    TOKEN_PUNCT,
    TOKEN_STRING,
)
from scripts.syntax_catalogue_bigquery.exceptions import BigQueryCatalogueError
from scripts.syntax_catalogue_bigquery.models import GrammarRule
from scripts.syntax_catalogue_bigquery.types import Node, TmToken

_END: TmToken = (TOKEN_END, "", 0)


def _skip_quoted(*, text: str, start: int) -> int:
    """The index after the quoted string or character literal opening at `start`."""
    quote: str = text[start]
    index: int = start + 1
    while index < len(text) and text[index] != quote:
        index += 2 if text[index] == TM_ESCAPE else 1
    return index + 1


def _line_end(*, text: str, start: int) -> int:
    """The index of the newline ending the line at `start`, or the text's end."""
    end: int = text.find("\n", start)
    return len(text) if end < 0 else end


def _skip_action(*, text: str, start: int) -> int:
    """The index after the balanced `{ ... }` at `start`, C++ strings and comments skipped."""
    depth: int = 0
    index: int = start
    while index < len(text):
        ch: str = text[index]
        if text.startswith(TM_LINE_COMMENT, index):
            index = _line_end(text=text, start=index)
        elif text.startswith(TM_BLOCK_OPEN, index):
            index = text.find(TM_BLOCK_CLOSE, index) + len(TM_BLOCK_CLOSE)
        elif ch in TM_QUOTES:
            index = _skip_quoted(text=text, start=index)
        else:
            depth += (ch == TM_ACTION_OPEN) - (ch == TM_ACTION_CLOSE)
            index += 1
            if depth == 0:
                return index
    raise BigQueryCatalogueError(f"{GRAMMAR_FILE}: unbalanced action at offset {start}")


def _token_at(*, text: str, index: int) -> tuple[TmToken | None, int]:
    """The token starting at `index` (None for space, comments, labels, markers), and its end."""
    ch: str = text[index]
    if ch.isspace():
        return None, index + 1
    if text.startswith(TM_LINE_COMMENT, index) or ch == TM_HASH_COMMENT:
        return None, _line_end(text=text, start=index)
    if text.startswith(TM_BLOCK_OPEN, index):
        return None, text.find(TM_BLOCK_CLOSE, index) + len(TM_BLOCK_CLOSE)
    if ch == TM_STRING_QUOTE:
        end: int = _skip_quoted(text=text, start=index)
        return (TOKEN_STRING, text[index + 1 : end - 1], 0), end
    if ch == TM_ACTION_OPEN:
        end = _skip_action(text=text, start=index)
        return (TOKEN_ACTION, text[index:end], 0), end
    if ch == TM_LABEL_OPEN:
        return None, text.index(TM_LABEL_CLOSE, index) + 1
    directive: re.Match[str] | None = TM_DIRECTIVE.match(text, index)
    if directive:
        return (TOKEN_DIRECTIVE, directive.group(0), 0), directive.end()
    marker: re.Match[str] | None = TM_NAME.match(text, index + 1) if ch == TM_MARKER else None
    if marker:
        return None, marker.end()
    name: re.Match[str] | None = TM_NAME.match(text, index)
    if name:
        return (TOKEN_NAME, name.group(0), 0), name.end()
    return (TOKEN_PUNCT, ch, 0), index + 1


def tokens(text: str) -> list[TmToken]:
    """(kind, text, line) for every token of the parser section; actions as one token each."""
    found: list[TmToken] = []
    index: int = 0
    line: int = 1
    counted: int = 0
    while index < len(text):
        line += text.count("\n", counted, index)
        counted = index
        token, index = _token_at(text=text, index=index)
        if token is not None:
            found.append((token[0], token[1], line))
    return found


def helper_features(text: str) -> dict[str, tuple[str, ...]]:
    """Helper functions of the parser template (after `%%`) and the features each one tests."""
    tail: str = text[text.find(TM_END) :]
    found: dict[str, list[str]] = {}
    for match in FEATURE_TEST.finditer(tail):
        heads: list[str] = HELPER_HEAD.findall(tail[: match.start()])
        if heads:
            found.setdefault(heads[-1], []).append(match.group(0))
    return {name: tuple(dict.fromkeys(features)) for name, features in found.items()}


class _Parser:
    """A recursive-descent reader of rule right-hand sides."""

    def __init__(self, items: list[TmToken], helpers: dict[str, tuple[str, ...]]) -> None:
        self.items: list[TmToken] = items
        self.helpers: dict[str, tuple[str, ...]] = helpers
        self.index: int = 0

    def peek(self) -> TmToken:
        """The next token, or an end marker."""
        return self.items[self.index] if self.index < len(self.items) else _END

    def take(self) -> TmToken:
        """Consume the next token."""
        token: TmToken = self.peek()
        self.index += 1
        return token

    def alternatives(self, *, closer: str) -> tuple[list[Node], list[tuple[str, ...]]]:
        """Alternatives up to `closer`, each with the features (or REJECTED) its actions test."""
        found: list[Node] = []
        features: list[tuple[str, ...]] = []
        items: list[Node] = []
        tested: list[str] = []
        while True:
            kind, text, _ = self.peek()
            if kind == TOKEN_END:
                raise BigQueryCatalogueError(f"{GRAMMAR_FILE}: unterminated rule")
            if kind == TOKEN_PUNCT and text in (closer, TM_ALTERNATIVE):
                found.append((NODE_SEQ, items))
                features.append(tuple(dict.fromkeys(tested)))
                items, tested = [], []
                self.take()
                if text == closer:
                    return found, features
                continue
            if kind == TOKEN_PUNCT and closer == TM_GROUP_CLOSE and text == TM_RULE_END:
                raise BigQueryCatalogueError(f"{GRAMMAR_FILE}: unclosed group")
            item, found_features = self.item()
            tested.extend(found_features)
            if item is not None:
                items.append(self.suffixed(item))

    def action(self, text: str) -> list[str]:
        """The features an action tests, directly or through a template helper."""
        tested: list[str] = FEATURE_TEST.findall(text)
        for helper, features in self.helpers.items():
            if f"{helper}(" in text:
                tested.extend(features)
        if REJECTING_ACTION.match(text):
            tested.append(REJECTED)
        return tested

    def skip_set(self) -> Node:
        """A `set(...)` token class, read to its closing parenthesis."""
        depth: int = 0
        while True:
            kind, text, _ = self.take()
            if kind == TOKEN_END:
                raise BigQueryCatalogueError(f"{GRAMMAR_FILE}: unclosed set(...)")
            if kind == TOKEN_PUNCT:
                depth += (text == TM_GROUP_OPEN) - (text == TM_GROUP_CLOSE)
            if depth == 0:
                return (NODE_SET,)

    def separator(self) -> Node:
        """The separator words of a `(x separator ",")` list, up to the closing parenthesis."""
        words: list[str] = []
        while self.peek()[1] != TM_GROUP_CLOSE:
            words.append(self.take()[1])
        return (SEPARATOR, " ".join(words))

    def group(self) -> tuple[Node, list[str]]:
        """A parenthesised list, choice or sequence, and the features its actions test."""
        body, features = self.alternatives(closer=TM_GROUP_CLOSE)
        tested: list[str] = list(itertools.chain.from_iterable(features))
        members: list[Node] = list(body[0][1]) if len(body) == 1 else []  # type: ignore[arg-type]
        if members and members[-1][0] == SEPARATOR:
            separator: Node = (NODE_LIT, str(members[-1][1]))
            return (NODE_LIST, (NODE_SEQ, members[:-1]), separator), tested
        return ((NODE_CHOICE, body) if len(body) > 1 else body[0]), tested

    def item(self) -> tuple[Node | None, list[str]]:
        """One symbol, group or set (None for an action or directive), and the features it tests."""
        kind, text, _ = self.take()
        if kind == TOKEN_ACTION:
            return None, self.action(text)
        if kind == TOKEN_PUNCT and text == TM_GROUP_OPEN:
            return self.group()
        return self.plain(kind=kind, text=text), []

    def plain(self, *, kind: str, text: str) -> Node | None:
        """One symbol or set; None for a directive."""
        if kind == TOKEN_DIRECTIVE:
            if text == TM_PREC:
                self.take()
            return None
        if kind == TOKEN_STRING:
            return (NODE_LIT, text)
        if kind == TOKEN_NAME and text == SEPARATOR:
            return self.separator()
        if kind == TOKEN_NAME and text == NODE_SET and self.peek()[1] == TM_GROUP_OPEN:
            return self.skip_set()
        if kind == TOKEN_NAME and text.endswith(OPT_SUFFIX):
            return (NODE_OPT, symbol(text.removesuffix(OPT_SUFFIX)))
        if kind == TOKEN_NAME:
            return symbol(text)
        raise BigQueryCatalogueError(f"{GRAMMAR_FILE}: unexpected {kind} {text!r}")

    def suffixed(self, item: Node) -> Node:
        """The item with its `?`, `*` or `+` wrapper."""
        while self.peek()[0] == TOKEN_PUNCT and self.peek()[1] in TM_WRAPPERS:
            item = (TM_WRAPPERS[self.take()[1]], item)
        return item


def symbol(name: str) -> Node:
    """A rule reference, or a lexer token such as IDENTIFIER or KW_EXCEPT_IN_SET_OP."""
    return (NODE_TOKEN, name) if TM_TOKEN_NAME.fullmatch(name) else (NODE_REF, name)


def parser_section(text: str) -> tuple[str, int]:
    """The text between `:: parser` and the closing `%%`, and the file line it starts on."""
    start: int = text.find(TM_PARSER)
    end: int = text.find(TM_END, start)
    if start < 0 or end < 0:
        raise BigQueryCatalogueError(f"{GRAMMAR_FILE}: parser section not found")
    return text[start + len(TM_PARSER) : end], text.count("\n", 0, start)


def read_rules(cache: Path) -> dict[str, GrammarRule]:
    """Every parser rule of the pinned grammar, with its alternatives' feature tests."""
    whole: str = _grammar_text(cache)
    section, offset = parser_section(whole)
    items: list[TmToken] = [(kind, text, line + offset) for kind, text, line in tokens(section)]
    parser: _Parser = _Parser(items, helper_features(whole))
    rules: dict[str, GrammarRule] = {}
    while parser.peek()[0] != TOKEN_END:
        kind, text, line = parser.take()
        if kind == TOKEN_DIRECTIVE:
            while parser.peek()[1] != TM_RULE_END:
                parser.take()
            parser.take()
            continue
        if kind != TOKEN_NAME:
            raise BigQueryCatalogueError(f"{GRAMMAR_FILE}:{line}: expected a rule, got {text!r}")
        while parser.peek()[0] == TOKEN_ACTION:
            parser.take()
        if parser.take()[1] != TM_RULE_COLON:
            raise BigQueryCatalogueError(f"{GRAMMAR_FILE}:{line}: rule {text} has no ':'")
        body, features = parser.alternatives(closer=TM_RULE_END)
        rules[text] = GrammarRule(
            name=text, body=(NODE_CHOICE, body), features=tuple(features), line=line
        )
    return rules


def _grammar_text(cache: Path) -> str:
    """The pinned googlesql.tm."""
    return (cache / GRAMMAR_DIR / GRAMMAR_FILE).read_text()


def dispatched_words(cache: Path) -> dict[str, int]:
    """Names the actions compare identifiers with (CaseEqual), by line; prefixes joined."""
    section, offset = parser_section(_grammar_text(cache))
    found: dict[str, int] = {}
    for kind, text, line in tokens(section):
        if kind != TOKEN_ACTION:
            continue
        words: list[str] = DISPATCH_CALL.findall(text)
        prefixes: list[str] = [w for w in words if w.endswith(DISPATCH_PREFIX)]
        for word in words:
            names: list[str] = [f"{p}{word}" for p in prefixes] or [word]
            if word not in prefixes:
                found.update({n.upper(): line + offset for n in names if n.upper() not in found})
    return found
