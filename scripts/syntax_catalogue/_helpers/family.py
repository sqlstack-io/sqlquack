"""Page families, source ids and id bases from a page's cache path and title."""

from __future__ import annotations

import re

from scripts.syntax_catalogue._helpers.grammar import slug
from scripts.syntax_catalogue.constants import (
    ADMIN_WORDS,
    CALL_DIRECTORIES,
    CLASSES_DIRECTORY,
    COMMANDS_DIRECTORY,
    CONSTRAINTS_PREFIX,
    CONSTRUCTS_DIRECTORY,
    DML_TITLES,
    LEADING_EXTRA,
    MARKDOWN_SUFFIX,
    QUERY_PAGES,
    QUERY_TITLES,
    REFERENCE_ROOT,
    SCRIPTING_DIRECTORY,
    SESSION_TITLES,
    TOP_LEVEL_PARTS,
    UTILITY_TITLES,
    VIEW_DIRECTORIES,
    WINDOW_SYNTAX_PREFIX,
)


def command_family(title: str) -> str:
    """The family of a SQL command page, from its title."""
    text: str = re.sub(r"\s+", " ", title).strip()
    for pattern, family in (
        (QUERY_TITLES, "query"),
        (SESSION_TITLES, "transaction_session"),
        (DML_TITLES, "dml"),
        (UTILITY_TITLES, "utility"),
    ):
        if pattern.match(text):
            return family
    return "security_admin" if ADMIN_WORDS.search(text) else "ddl"


def family_of(path: str, title: str) -> str | None:
    """The page's family, or None for pages outside the syntax reference."""
    parts: list[str] = path.removesuffix(MARKDOWN_SUFFIX).split("/")
    if parts[0] != REFERENCE_ROOT or len(parts) < TOP_LEVEL_PARTS:
        return None
    if len(parts) == TOP_LEVEL_PARTS:
        name: str = parts[1]
        if name in QUERY_PAGES:
            return "query"
        if name.startswith(WINDOW_SYNTAX_PREFIX):
            return "functions"
        return "ddl" if name.startswith(CONSTRAINTS_PREFIX) else "expressions"
    directory: str = parts[1]
    if directory == COMMANDS_DIRECTORY:
        return command_family(title)
    if directory in CALL_DIRECTORIES or directory in VIEW_DIRECTORIES:
        return "functions"
    if directory == CLASSES_DIRECTORY:
        return "classes"
    if directory == SCRIPTING_DIRECTORY:
        return "scripting"
    return "query" if directory == CONSTRUCTS_DIRECTORY else None


def is_view_page(path: str) -> bool:
    """True for INFORMATION_SCHEMA, ACCOUNT_USAGE and ORGANIZATION_USAGE pages."""
    parts: list[str] = path.split("/")
    return len(parts) > TOP_LEVEL_PARTS and parts[1] in VIEW_DIRECTORIES


def source_id(path: str) -> str:
    """The catalogue id of a page: `doc.` plus its path under sql-reference, dot-separated."""
    trimmed: str = path.removesuffix(MARKDOWN_SUFFIX).removeprefix(REFERENCE_ROOT + "/")
    return "doc." + trimmed.replace("/", ".")


def page_slug(path: str) -> str:
    """The id base segment of a page."""
    parts: list[str] = path.removesuffix(MARKDOWN_SUFFIX).split("/")
    return ".".join(slug(p) for p in parts[2:]) if len(parts) > TOP_LEVEL_PARTS else slug(parts[-1])


def leading_words(titles: list[str]) -> set[str]:
    """Words a statement on a command page may start with: each command title's first word."""
    return {t.split()[0].upper() for t in titles if t} | LEADING_EXTRA
