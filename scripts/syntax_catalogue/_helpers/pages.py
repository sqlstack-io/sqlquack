"""Read one documentation page (Markdown) into its sqlsyntax blocks and classified productions."""

from __future__ import annotations

import html
import re

from scripts.syntax_catalogue._helpers.grammar import bare_words, split_productions
from scripts.syntax_catalogue.constants import (
    BULLET_CHUNK,
    DEFINITION,
    ELISION,
    ELISION_LINE,
    FENCE,
    GRAMMAR_MARKS,
    H1,
    HEADING,
    KIND_DEFINITION,
    KIND_FRAGMENT,
    KIND_STATEMENT,
    LABEL_CHUNK,
    MIN_VARIANT_DEPTH,
    NON_FORM_SECTIONS,
    PILCROW_STRIP,
    PLAIN_FENCE,
    SYNTAX_FENCE,
    SYNTAX_HEADING,
    SYNTAX_TITLE,
    TAG,
    TOP_LEVEL_PARTS,
)
from scripts.syntax_catalogue.models import Block, Page, Production


def parse_page(markdown: str) -> Page:
    """Headings and fenced `sqlsyntax` blocks, each with the heading path above it."""
    title: str = ""
    path: list[tuple[int, str]] = []
    blocks: list[Block] = []
    fence: tuple[str, str] | None = None
    buffer: list[str] = []
    heading_has_block: dict[tuple[str, ...], bool] = {}
    for line in markdown.splitlines():
        fence_match: re.Match[str] | None = FENCE.match(line)
        if fence is None and fence_match:
            language: str = (fence_match.group(2) or PLAIN_FENCE).lower()
            if SYNTAX_TITLE.search(fence_match.group(3)):
                language = SYNTAX_FENCE
            fence = (fence_match.group(1)[:3], language)
            buffer = []
            continue
        if fence is not None:
            closing: bool = bool(fence_match) and not (fence_match.group(2) or fence_match.group(3))
            if fence_match and closing and fence_match.group(1)[:3] == fence[0]:
                if fence[1] == SYNTAX_FENCE:
                    key: tuple[str, ...] = tuple(text for _, text in path)
                    blocks.append(Block(key, "\n".join(buffer)))
                    heading_has_block[key] = True
                fence = None
            else:
                buffer.append(line)
            continue
        heading: re.Match[str] | None = HEADING.match(line)
        if heading:
            level: int = len(heading.group(1))
            text: str = heading.group(2).strip()
            if level == 1 and not title:
                title = text
            path = [(lv, t) for lv, t in path if lv < level] + [(level, text)]
            heading_has_block.setdefault(tuple(t for _, t in path), False)
    return Page(title, tuple(blocks), tuple(headings_without_block(heading_has_block)))


def headings_without_block(heading_has_block: dict[tuple[str, ...], bool]) -> list[str]:
    """Headings inside a Syntax/Variant syntax section with no block of their own or below."""
    missing: list[str] = []
    for key, has in heading_has_block.items():
        if has or len(key) < MIN_VARIANT_DEPTH or not SYNTAX_HEADING.search(key[1]):
            continue
        if not any(k[: len(key)] == key and v for k, v in heading_has_block.items()):
            missing.append(key[-1])
    return missing


def in_form_section(*, heading: tuple[str, ...], syntax_sections_only: bool) -> bool:
    """True when a block under this heading documents a form rather than a fragment."""
    if any(NON_FORM_SECTIONS.match(h) for h in heading[1:]):
        return False
    if not syntax_sections_only or len(heading) < TOP_LEVEL_PARTS:
        return True
    return bool(SYNTAX_HEADING.search(heading[1]))


def is_non_grammar(text: str) -> bool:
    """True for a list item, a lone label such as `Aggregate:`, or a keyword-order illustration."""
    illustration: bool = ELISION in text and not any(ch in text for ch in GRAMMAR_MARKS)
    return bool(BULLET_CHUNK.match(text) or LABEL_CHUNK.match(text) or illustration)


def block_productions(
    *, block: Block, leading_words: set[str] | None, form_section: bool
) -> list[Production]:
    """Classify one block's chunks; a chunk that starts no statement continues the one before."""
    pending: list[tuple[str, list[str], str]] = []
    for text in split_productions(block.text):
        definition: re.Match[str] | None = DEFINITION.match(text)
        if definition:
            pending.append((KIND_DEFINITION, [text], definition.group(1).strip()))
            continue
        first: str = (bare_words(text) or [""])[0].upper()
        starts: bool = leading_words is None or first in leading_words
        continues: bool = bool(pending) and (
            ELISION_LINE.match(text) is not None or pending[-1][0] == KIND_DEFINITION or not starts
        )
        if continues:
            pending[-1][1].append(text)
            continue
        grammar_chunk: bool = form_section and starts and not is_non_grammar(text)
        pending.append((KIND_STATEMENT if grammar_chunk else KIND_FRAGMENT, [text], ""))
    return [Production(kind, " ".join(parts), block.heading, name) for kind, parts, name in pending]


def productions(
    *, page: Page, leading_words: set[str] | None, syntax_sections_only: bool
) -> list[Production]:
    """Classify each production of each block as a definition, statement or fragment."""
    result: list[Production] = []
    for block in page.blocks:
        section: bool = in_form_section(
            heading=block.heading, syntax_sections_only=syntax_sections_only
        )
        result += block_productions(block=block, leading_words=leading_words, form_section=section)
    return result


def html_title(page: str) -> str:
    """The text of an HTML page's first h1."""
    match: re.Match[str] | None = H1.search(page)
    return html.unescape(TAG.sub("", match.group(1))).strip(PILCROW_STRIP) if match else ""
