"""Read documented call signatures and PRAGMA spellings into the catalogue notation."""

from __future__ import annotations

import html
import re

from scripts.syntax_catalogue_sqlite._helpers.pages import text_of
from scripts.syntax_catalogue_sqlite.constants import (
    ARGUMENT_NAME,
    BOLD_CALL_SIGNATURES,
    CALL,
    CALL_STAR,
    DT_SIGNATURES,
    ELLIPSIS,
    EMPTY_PARENS,
    HEADING_SIGNATURES,
    ITALIC,
    ITALIC_ARGUMENT,
    ITALIC_CLOSE,
    ITALIC_OPEN,
    ITALIC_SPLIT,
    LI_SIGNATURES,
    NOTATION_BAR,
    OPERATOR_SIGNATURE,
    PLACEHOLDER_CHOICE,
    PRAGMA_ANCHOR,
    PRAGMA_BLOCK,
    PRAGMA_KEYWORD,
    PRAGMA_SCHEMA,
    PRAGMA_TOKEN,
    SCHEMA_ITALICS,
    SCHEMA_SEPARATOR,
    SIGNATURE_BREAK,
    SPACES,
    STATEMENT_END,
    TAG,
    WORD_START,
)
from scripts.syntax_catalogue_sqlite.models import Signature


def signatures(*, text: str) -> list[Signature]:
    """Every signature a function page documents, in page order."""
    found: list[Signature] = []
    for pattern in (DT_SIGNATURES, LI_SIGNATURES):
        for block in pattern.findall(text):
            for piece in SIGNATURE_BREAK.split(block):
                found += _signature(text_of(piece))
    for name, rest in BOLD_CALL_SIGNATURES.findall(text):
        found += _signature(text_of(name + rest + ")"))
    for heading in HEADING_SIGNATURES.findall(text):
        found += _signature(text_of(heading))
    return found


def _signature(text: str) -> list[Signature]:
    """One signature, or none when the text is not a call or an operator."""
    text = text.strip()
    match: re.Match[str] | None = CALL.match(text)
    if match is not None:
        return [Signature(match.group(1).lower(), text, match.group(1).lower())]
    operator: re.Match[str] | None = OPERATOR_SIGNATURE.match(text)
    if operator is not None:
        return [Signature(operator.group(2), text, operator.group(2))]
    return []


def call_grammar(text: str) -> str:
    """`abs(X)` as `ABS ( <x> )`; `f(X,Y,...)` as `F ( <x> , <y> [ , ... ] )`."""
    operator: re.Match[str] | None = OPERATOR_SIGNATURE.match(text)
    if operator is not None:
        left, symbol, right = operator.groups()
        return f"<{left}> {symbol} <{right}>"
    match: re.Match[str] | None = CALL.match(text)
    if match is None:
        return ""
    name: str = match.group(1).upper()
    arguments: list[str] = [a.strip() for a in match.group(2).split(",") if a.strip()]
    parts: list[str] = []
    for argument in arguments:
        if argument == ELLIPSIS:
            parts.append(f"[ , {ELLIPSIS} ]")
            break
        if argument == CALL_STAR:
            parts.append(CALL_STAR)
        elif ARGUMENT_NAME.match(argument):
            parts.append(("" if not parts else ", ") + f"<{argument.lower()}>")
        else:
            return ""
    inner: str = " ".join(parts)
    return f"{name} ( {inner} )" if inner else f"{name} {EMPTY_PARENS}"


def pragma_sections(text: str) -> list[tuple[str, str]]:
    """Each PRAGMA's name and the HTML of its section, in page order."""
    parts: list[str] = PRAGMA_ANCHOR.split(text)
    return [(parts[index], parts[index + 1]) for index in range(1, len(parts), 2)]


def pragma_lines(section: str) -> list[str]:
    """The spellings of a PRAGMA's syntax paragraph, italic parts marked."""
    match: re.Match[str] | None = PRAGMA_BLOCK.search(section)
    if match is None:
        return []
    marked: str = ITALIC.sub(lambda m: ITALIC_OPEN + m.group(1) + ITALIC_CLOSE, match.group(1))
    lines: list[str] = []
    for piece in SIGNATURE_BREAK.split(marked):
        line: str = html.unescape(TAG.sub("", piece)).replace("\xa0", " ")
        line = SPACES.sub(" ", line).strip()
        if line.startswith(PRAGMA_KEYWORD):
            lines.append(line)
    return lines


def _italic(content: str) -> str:
    """An italic part of a PRAGMA spelling in the catalogue notation."""
    content = content.strip()
    if content in SCHEMA_ITALICS:
        return PRAGMA_SCHEMA
    argument: re.Match[str] | None = ITALIC_ARGUMENT.fullmatch(content)
    if argument is not None:
        return f"( <{argument.group(1).lower()}> )"
    if NOTATION_BAR in content:
        values: list[str] = [v.strip().upper() for v in content.split(NOTATION_BAR) if v.strip()]
        return "{ " + f" {NOTATION_BAR} ".join(values) + " }"
    return "<" + SPACES.sub("-", content.lower()) + ">"


def pragma_grammar(line: str) -> str:
    """One PRAGMA spelling in the catalogue notation, its pragma name upper-cased."""
    out: list[str] = []
    after_schema: bool = False
    for index, chunk in enumerate(ITALIC_SPLIT.split(line)):
        if index % 2:
            after_schema = _italic(chunk) == PRAGMA_SCHEMA
            out.append(_italic(chunk))
            continue
        text: str = chunk.replace(STATEMENT_END, " ")
        if after_schema:
            text = text.lstrip().removeprefix(SCHEMA_SEPARATOR)
        after_schema = False
        for token in PRAGMA_TOKEN.findall(text):
            out.append(token.upper() if WORD_START.match(token) else token)
    joined: str = PLACEHOLDER_CHOICE.sub(r"{ \1 | \2 }", " ".join(out))
    return SPACES.sub(" ", joined.replace("(  )", EMPTY_PARENS)).strip()
