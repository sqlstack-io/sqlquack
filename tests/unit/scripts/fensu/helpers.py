"""Run the single-rule rewrites over a source string."""

from __future__ import annotations

from scripts.fensu._helpers.rewrites.derives import debug_edits
from scripts.fensu._helpers.rewrites.literals import compared_literals, literal_edits
from scripts.fensu._helpers.rewrites.unreachable import unreachable_edits
from scripts.fensu._helpers.rewrites.unwraps import unwrap_edits
from scripts.fensu._helpers.rust.comments import apply_edits
from scripts.fensu._helpers.rust.items import parse_items
from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu.models import Item, Token


def derived(source: str) -> str:
    """`source` with every model deriving `Debug`."""
    items: list[Item] = parse_items(
        text=source, tokens=tokenize(text=source), start=0, end=len(source)
    )
    return apply_edits(text=source, edits=debug_edits(text=source, items=items))


def without_unreachable(source: str) -> str:
    """`source` with `unreachable!` in `Result` functions rewritten."""
    return apply_edits(
        text=source, edits=unreachable_edits(text=source, tokens=tokenize(text=source))
    )


def without_unwraps(source: str) -> str:
    """`source` with unwraps in `Result` functions rewritten."""
    return apply_edits(text=source, edits=unwrap_edits(text=source, tokens=tokenize(text=source)))


def named_literals(source: str) -> str:
    """`source` with each compared string literal replaced by its constant's name."""
    literals: list[Token] = compared_literals(tokens=tokenize(text=source))
    return apply_edits(text=source, edits=literal_edits(literals=literals))
