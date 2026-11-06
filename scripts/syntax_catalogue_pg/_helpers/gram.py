"""Read PostgreSQL's bison grammar and keyword list: rules, their alternatives and keywords."""

from __future__ import annotations

import re
from pathlib import Path

from scripts.syntax_catalogue_pg.models import Keyword, Rule

RULES_START: str = "\n%%\n"
KEYWORD_LINE: re.Pattern[str] = re.compile(r'^PG_KEYWORD\("([^"]+)", (\w+), (\w+), (\w+)\)', re.M)
RULE_HEAD: re.Pattern[str] = re.compile(r"^([a-zA-Z_]\w*):", re.M)
TOKEN: re.Pattern[str] = re.compile(r"'(?:\\.|[^'])'|[A-Za-z_]\w*|%prec\s+\w+")
PUNCTUATION_TOKENS: dict[str, str] = {
    "TYPECAST": "::",
    "DOT_DOT": "..",
    "COLON_EQUALS": ":=",
    "EQUALS_GREATER": "=>",
    "LESS_EQUALS": "<=",
    "GREATER_EQUALS": ">=",
    "NOT_EQUALS": "<>",
}


def keywords(path: Path) -> list[Keyword]:
    """The keywords kwlist.h declares, with their token name and category."""
    return [
        Keyword(text=word.upper(), token=token, category=category)
        for word, token, category, _ in KEYWORD_LINE.findall(path.read_text())
    ]


def strip_code(text: str) -> str:
    """The rules section without C comments, action blocks and character-literal braces."""
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    out: list[str] = []
    depth: int = 0
    index: int = 0
    while index < len(text):
        ch: str = text[index]
        if ch == "'" and index + 2 < len(text) and text[index + 2] == "'" and depth == 0:
            out.append(text[index : index + 3])
            index += 3
            continue
        if ch in "\"'" and depth > 0:
            end: int = index + 1
            while end < len(text) and text[end] != ch:
                end += 2 if text[end] == "\\" else 1
            index = end + 1
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif depth == 0:
            out.append(ch)
        index += 1
    return "".join(out)


def rules(path: Path) -> dict[str, Rule]:
    """Every rule of gram.y with its alternatives as token lists, in file order."""
    text: str = path.read_text()
    body: str = strip_code(text.split(RULES_START, 2)[1])
    heads: list[re.Match[str]] = list(RULE_HEAD.finditer(body))
    found: dict[str, Rule] = {}
    for head, following in zip(heads, [*heads[1:], None], strict=True):
        end: int = following.start() if following else len(body)
        definition: str = body[head.end() : end].rsplit(";", 1)[0]
        alternatives: list[tuple[str, ...]] = [
            tuple(t for t in TOKEN.findall(part) if not t.startswith("%prec"))
            for part in definition.split("|")
        ]
        found[head.group(1)] = Rule(name=head.group(1), alternatives=tuple(alternatives))
    return found


def token_text(token: str, keyword_tokens: dict[str, str]) -> str:
    """A grammar token as SQL text: a keyword, a punctuation token or a quoted character."""
    if token in keyword_tokens:
        return keyword_tokens[token]
    if token in PUNCTUATION_TOKENS:
        return PUNCTUATION_TOKENS[token]
    if token.startswith("'"):
        return token.strip("'").replace("\\", "")
    return ""
