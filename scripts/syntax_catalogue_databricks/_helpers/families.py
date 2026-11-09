"""Which family a reference page's forms belong to, and the slug its form ids start from."""

from __future__ import annotations

from scripts.grammar_catalogue._helpers.grammar import slug
from scripts.syntax_catalogue_databricks.constants import (
    EXPRESSIONS,
    FAMILY_BY_PAGE,
    FAMILY_BY_PREFIX,
    SLUG_PREFIXES,
    SOURCE_PREFIX,
)


def family_of(path: str) -> str:
    """The family of a page: by its exact path, else by its longest listed prefix."""
    if path in FAMILY_BY_PAGE:
        return FAMILY_BY_PAGE[path]
    owners: list[str] = [prefix for prefix in FAMILY_BY_PREFIX if path.startswith(prefix)]
    if not owners:
        return EXPRESSIONS
    return FAMILY_BY_PREFIX[max(owners, key=len)]


def page_slug(path: str) -> str:
    """The id slug of a page: its last path part without the reference's category prefix."""
    last: str = path.rsplit("/", 1)[-1]
    for prefix in SLUG_PREFIXES:
        if last.startswith(prefix):
            last = last.removeprefix(prefix)
            break
    return slug(last) or "index"


def source_id(path: str) -> str:
    """`doc.<path with / as .>`, or `doc.index` for the manual's own page."""
    return SOURCE_PREFIX + (path.replace("/", ".") or "index")
