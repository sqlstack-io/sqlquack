"""The example blocks the homepage and the docs pages show, and the blocks no marker names."""

from __future__ import annotations

import html
from pathlib import Path

from scripts.website_examples.constants import (
    COMMAND_SPAN,
    DOCS_DIR,
    HOMEPAGE,
    MDX_BLOCK,
    MDX_TRANSCRIPT_LANGUAGE,
    PROMPT,
    TAG,
    TERMINAL_BLOCK,
)
from scripts.website_examples.models import ShownBlock


def shown_blocks(root: Path) -> tuple[list[ShownBlock], list[str]]:
    """Every marked example block, and a problem for each terminal or console block unmarked."""
    found: list[ShownBlock] = []
    homepage: str = (root / HOMEPAGE).read_text(encoding="utf-8")
    for match in TERMINAL_BLOCK.finditer(homepage):
        line: int = homepage.count("\n", 0, match.start()) + 1
        text: str = html.unescape(TAG.sub("", COMMAND_SPAN.sub(PROMPT + r"\1", match["body"])))
        found.append(ShownBlock(HOMEPAGE, line, *_named(match["spec"]), text))
    for path in sorted((root / DOCS_DIR).rglob("*.mdx")):
        page: str = path.relative_to(root).as_posix()
        source: str = path.read_text(encoding="utf-8")
        for match in MDX_BLOCK.finditer(source):
            if match["spec"] is None and match["language"] != MDX_TRANSCRIPT_LANGUAGE:
                continue
            line = source.count("\n", 0, match.start()) + 1
            text = match["body"].removesuffix("\n")
            found.append(ShownBlock(page, line, *_named(match["spec"]), text))
    unmarked: list[str] = [
        f"{block.page}:{block.line}: the block names no example"
        for block in found
        if not block.example
    ]
    return [block for block in found if block.example], unmarked


def _named(spec: str | None) -> tuple[str, str]:
    """The example a marker names and the rest of its specification; empty without a marker."""
    name, _, rest = (spec or "").partition(" ")
    return name, rest
