"""Read parse.y as the pinned amalgamation compiles it, and mkkeywordhash.c's keyword table."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.syntax_catalogue_sqlite._helpers.fetch import archive_path, member
from scripts.syntax_catalogue_sqlite.constants import (
    BLOCK_COMMENT,
    BRACE_CLOSE,
    BRACE_OPEN,
    C_QUOTES,
    DIRECTIVE_ELSE,
    DIRECTIVE_ENDIF,
    DIRECTIVE_IF,
    DIRECTIVE_IFDEF,
    DIRECTIVE_IFNDEF,
    DIRECTIVE_LINE,
    ESCAPE,
    FALLBACK,
    GRAMMAR_DEFINES,
    GRAMMAR_FILE,
    GRAMMAR_RULE,
    GRAMMAR_SYMBOL,
    IF_AND,
    IF_NOT,
    IF_OPEN,
    IF_OR,
    IF_TOKEN,
    KEYWORD_FILE,
    KEYWORD_ROW,
    KEYWORD_TABLE,
    LINE_COMMENT,
    NEWLINE,
    SOURCE,
    SOURCE_PREFIX,
    START_RULE,
    TOKEN_CLASS,
    WILDCARD,
)
from scripts.syntax_catalogue_sqlite.exceptions import SqliteCatalogueError
from scripts.syntax_catalogue_sqlite.models import Grammar, Keyword, Production


def source_text(*, cache: Path, name: str) -> str:
    """One pinned file of the source archive, as text."""
    archive: Path = archive_path(cache=cache, name=SOURCE)
    return member(archive=archive, name=SOURCE_PREFIX + name).decode()


def blanked(text: str) -> str:
    """The text with the same line breaks and only the newlines of a removed span kept."""
    return NEWLINE * text.count(NEWLINE)


def without_comments(text: str) -> str:
    """parse.y with `//` and `/* */` comments blanked, line numbers kept."""
    text = BLOCK_COMMENT.sub(lambda m: blanked(m.group(0)), text)
    return LINE_COMMENT.sub("", text)


def condition_true(*, directive: str, operand: str) -> bool:
    """Whether a Lemon conditional holds with the GRAMMAR_DEFINES of the amalgamation build."""
    if directive == DIRECTIVE_IFDEF:
        return operand in GRAMMAR_DEFINES
    if directive == DIRECTIVE_IFNDEF:
        return operand not in GRAMMAR_DEFINES
    tokens: list[str] = IF_TOKEN.findall(operand)
    value, rest = _either(tokens)
    if rest:
        raise SqliteCatalogueError(f"parse.y: cannot read %if {operand}")
    return value


def _either(tokens: list[str]) -> tuple[bool, list[str]]:
    """`a || b || ...` of a Lemon %if expression, and the tokens after it."""
    found: tuple[bool, list[str]] = _both(tokens)
    while found[1][:1] == [IF_OR]:
        right: tuple[bool, list[str]] = _both(found[1][1:])
        found = (found[0] or right[0], right[1])
    return found


def _both(tokens: list[str]) -> tuple[bool, list[str]]:
    """`a && b && ...` of a Lemon %if expression, and the tokens after it."""
    found: tuple[bool, list[str]] = _term(tokens)
    while found[1][:1] == [IF_AND]:
        right: tuple[bool, list[str]] = _term(found[1][1:])
        found = (found[0] and right[0], right[1])
    return found


def _term(tokens: list[str]) -> tuple[bool, list[str]]:
    """`!a`, `( ... )` or a macro name, which holds when GRAMMAR_DEFINES defines it."""
    if not tokens:
        raise SqliteCatalogueError("parse.y: %if expression ends early")
    if tokens[0] == IF_NOT:
        value, rest = _term(tokens[1:])
        return not value, rest
    if tokens[0] == IF_OPEN:
        value, rest = _either(tokens[1:])
        return value, rest[1:]
    return tokens[0] in GRAMMAR_DEFINES, tokens[1:]


def compiled(text: str) -> str:
    """The lines inside true conditionals only; directives and false branches blanked."""
    stack: list[tuple[bool, bool]] = []
    out: list[str] = []
    for line in text.split(NEWLINE):
        match: re.Match[str] | None = DIRECTIVE_LINE.match(line)
        active: bool = all(taken for taken, _ in stack)
        if match is None:
            out.append(line if active else "")
            continue
        directive, operand = match.group(1), match.group(2).strip()
        if directive in (DIRECTIVE_IFDEF, DIRECTIVE_IFNDEF, DIRECTIVE_IF):
            holds: bool = condition_true(directive=directive, operand=operand)
            stack.append((holds, holds))
        elif directive == DIRECTIVE_ELSE:
            _, held = stack.pop()
            stack.append((not held, True))
        elif directive == DIRECTIVE_ENDIF:
            stack.pop()
        out.append("")
    if stack:
        raise SqliteCatalogueError("parse.y: unbalanced %if directives")
    return NEWLINE.join(out)


def without_code(text: str) -> str:
    """The text with every `{ ... }` C code block blanked, C strings and characters respected."""
    out: list[str] = []
    depth: int = 0
    index: int = 0
    while index < len(text):
        char: str = text[index]
        if depth and char in C_QUOTES:
            end: int = index + 1
            while end < len(text) and text[end] != char:
                end += 2 if text[end] == ESCAPE else 1
            out.append(blanked(text[index : end + 1]))
            index = end + 1
            continue
        if char == BRACE_OPEN:
            depth += 1
        elif char == BRACE_CLOSE:
            depth -= 1
        elif not depth or char == NEWLINE:
            out.append(char)
        index += 1
    return "".join(out)


def read_grammar(cache: Path) -> Grammar:
    """parse.y's productions by rule, its terminals, token classes, fallbacks and wildcard."""
    text: str = without_code(
        compiled(without_comments(source_text(cache=cache, name=GRAMMAR_FILE)))
    )
    rules: dict[str, list[Production]] = {}
    for match in GRAMMAR_RULE.finditer(text):
        line: int = text.count(NEWLINE, 0, match.start()) + 1
        symbols: tuple[str, ...] = tuple(GRAMMAR_SYMBOL.findall(match.group(2)))
        rules.setdefault(match.group(1), []).append(Production(match.group(1), symbols, line))
    classes: dict[str, tuple[str, ...]] = {
        name: tuple(members.split("|")) for name, members in TOKEN_CLASS.findall(text)
    }
    fallback: list[str] = []
    for found in FALLBACK.findall(text):
        fallback += found.split()[1:]
    terminals: set[str] = set()
    for productions in rules.values():
        for production in productions:
            for symbol in production.symbols:
                terminals |= {part for part in symbol.split("|") if part[:1].isupper()}
    return Grammar(
        rules=rules,
        tokens=frozenset(terminals),
        start=START_RULE,
        token_classes=classes,
        fallback=frozenset(fallback),
        wildcard=frozenset(WILDCARD.findall(text)),
    )


def read_keywords(cache: Path) -> list[Keyword]:
    """mkkeywordhash.c's keyword table: each word and the token it scans as."""
    text: str = source_text(cache=cache, name=KEYWORD_FILE)
    table: re.Match[str] | None = KEYWORD_TABLE.search(text)
    if table is None:
        raise SqliteCatalogueError(f"{KEYWORD_FILE}: no aKeywordTable")
    return [Keyword(word, token) for word, token in KEYWORD_ROW.findall(table.group(1))]
