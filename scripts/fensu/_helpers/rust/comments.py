"""Removing non-doc comments: whole lines when alone on them, otherwise just the comment."""

from __future__ import annotations

from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu.constants import NON_DOC_COMMENT_KINDS
from scripts.fensu.models import Edit, Token


def strip_comments(*, text: str) -> str:
    """`text` without its non-doc comments."""
    return apply_edits(text=text, edits=comment_edits(text=text, tokens=tokenize(text=text)))


def comment_edits(*, text: str, tokens: list[Token]) -> list[Edit]:
    """The edits that remove every non-doc comment from `text`."""
    edits: list[Edit] = []
    for token in tokens:
        if token.kind not in NON_DOC_COMMENT_KINDS:
            continue
        line_start: int = text.rfind("\n", 0, token.start) + 1
        line_end: int = _line_end(text=text, position=token.end)
        before: str = text[line_start : token.start]
        after: str = text[token.end : line_end]
        if not before.strip() and not after.strip():
            edits.append(Edit(start=line_start, end=min(line_end + 1, len(text)), replacement=""))
        elif not after.strip():
            blank: int = len(before) - len(before.rstrip(" \t"))
            edits.append(Edit(start=token.start - blank, end=token.end, replacement=""))
        else:
            blank = len(after) - len(after.lstrip(" \t"))
            edits.append(Edit(start=token.start, end=token.end + blank, replacement=""))
    return edits


def apply_edits(*, text: str, edits: list[Edit]) -> str:
    """`text` with every edit applied; the edits must not overlap."""
    pieces: list[str] = []
    position: int = 0
    for edit in sorted(edits, key=lambda edit: edit.start):
        pieces.append(text[position : edit.start])
        pieces.append(edit.replacement)
        position = edit.end
    pieces.append(text[position:])
    return "".join(pieces)


def _line_end(*, text: str, position: int) -> int:
    end: int = text.find("\n", position)
    return len(text) if end < 0 else end
