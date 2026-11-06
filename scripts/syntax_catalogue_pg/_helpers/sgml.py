"""Read PostgreSQL's DocBook SGML: synopsis text, signatures and the sections around them."""

from __future__ import annotations

import bisect
import re

from scripts.syntax_catalogue._helpers.grammar import normalize
from scripts.syntax_catalogue_pg.constants import (
    COMMENT,
    ENTITIES,
    INDEXTERM,
    LITERAL_TAG,
    PARAM_TYPE,
    PARAMETER,
    PHRASE_KEEP,
    REPLACEABLE,
    RETURNVALUE,
    SECTION_TAG,
    SYNOPSIS_DIV,
    SYNOPSIS_HEADING,
    TAG,
    TITLE,
    TYPE_TAG,
)
from scripts.syntax_catalogue_pg.models import Section
from scripts.syntax_catalogue_pg.types import Spans


def entities(text: str) -> str:
    """The text with the entities the reference pages use replaced by their characters."""
    for entity, value in ENTITIES.items():
        text = text.replace(entity, value)
    return text


def plain(fragment: str) -> str:
    """A fragment's text without tags, on one line."""
    return normalize(entities(TAG.sub("", COMMENT.sub("", fragment))))


def synopsis_text(raw: str) -> str:
    """A synopsis block as grammar text: `<replaceable>x</replaceable>` becomes `<x>`."""
    text: str = RETURNVALUE.sub("", INDEXTERM.sub("", COMMENT.sub("", raw)))
    text = PARAM_TYPE.sub(lambda m: f"\x00{plain(m.group(1))} {plain(m.group(2))}\x01", text)
    text = TYPE_TAG.sub(lambda m: "\x00" + plain(m.group(1)) + "\x01", text)
    text = PARAMETER.sub(lambda m: "\x00" + plain(m.group(1)) + "\x01", text)
    text = REPLACEABLE.sub(lambda m: "\x00" + plain(m.group(1)) + "\x01", text)
    text = text.replace("<optional>", "[ ").replace("</optional>", " ]")
    text = PHRASE_KEEP.sub(lambda m: "\x02" + " ".join(m.group(1).split()) + "\x03", text)
    text = entities(TAG.sub("", text))
    return (
        text.replace("\x00", "<")
        .replace("\x01", ">")
        .replace("\x02", "<phrase>")
        .replace("\x03", "</phrase>")
    )


def signature_text(raw: str) -> str:
    """A function or operator signature as grammar text, without its return type."""
    text: str = RETURNVALUE.sub("", INDEXTERM.sub("", COMMENT.sub("", raw)))
    text = PARAM_TYPE.sub(lambda m: f"\x00{plain(m.group(1))} {plain(m.group(2))}\x01", text)
    text = TYPE_TAG.sub(lambda m: "\x00" + plain(m.group(1)) + "\x01", text)
    text = PARAMETER.sub(lambda m: "\x00" + plain(m.group(1)) + "\x01", text)
    text = REPLACEABLE.sub(lambda m: "\x00" + plain(m.group(1)) + "\x01", text)
    text = LITERAL_TAG.sub(lambda m: plain(m.group(1)), text)
    text = text.replace("<optional>", "[ ").replace("</optional>", " ]")
    text = normalize(entities(TAG.sub("", text)))
    return text.replace("\x00", "<").replace("\x01", ">")


def literals(raw: str) -> list[str]:
    """The `<literal>` texts of a signature, in order."""
    return [plain(m.group(1)) for m in LITERAL_TAG.finditer(raw)]


def section_at(text: str) -> Spans:
    """(start, end, enclosing sections) spans covering the file, outermost section first."""
    spans: Spans = []
    stack: list[Section] = []
    last: int = 0
    for match in SECTION_TAG.finditer(text):
        spans.append((last, match.start(), tuple(stack)))
        last = match.start()
        if match.group(1):
            if stack:
                stack.pop()
            continue
        title: re.Match[str] | None = TITLE.search(text, match.end())
        name: str = plain(title.group(1)) if title else ""
        if match.group(2) == SYNOPSIS_DIV:
            name = SYNOPSIS_HEADING
        stack.append(Section(tag=match.group(2), id=match.group(3) or "", title=name))
    spans.append((last, len(text), tuple(stack)))
    return spans


def sections_of(*, spans: Spans, offset: int) -> tuple[Section, ...]:
    """The sections enclosing an offset."""
    index: int = bisect.bisect_right([start for start, _, _ in spans], offset) - 1
    return spans[index][2] if index >= 0 else ()
