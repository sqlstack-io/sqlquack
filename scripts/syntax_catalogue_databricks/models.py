"""What the Databricks catalogue reads from a reference page."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class SyntaxBlock:
    """One code block of a page, with the heading it is under."""

    heading: str
    text: str


@dataclass(frozen=True)
class DbPage:
    """One pinned page of the SQL language reference."""

    url: str
    path: str
    title: str
    title_code: str
    page_sha256: str
    article_sha256: str
    syntax_sha256: str
    blocks: tuple[SyntaxBlock, ...]
    applies: tuple[str, ...]
    headings: tuple[str, ...]
    others: tuple[SyntaxBlock, ...]


@dataclass(frozen=True)
class RawProduction:
    """One production of a block: a statement (no name) or a named definition's body."""

    name: str
    text: str


@dataclass(frozen=True)
class HandForm:
    """A form catalogued by hand from a page whose syntax is in prose, bound to its article hash."""

    page: str
    anchor: str
    kind: str
    label: str
    grammars: tuple[str, ...]
    basis: str
    article_sha256: str


@dataclass(frozen=True)
class HandPage:
    """The reviewed status of a page that catalogues no form, bound to its article hash."""

    page: str
    status: str
    reason: str
    article_sha256: str


@dataclass(frozen=True)
class HandReading:
    """A converted production read as other lines: a typo fixed or an example dropped."""

    page: str
    written: str
    read: tuple[str, ...]
    reason: str


@dataclass(frozen=True)
class Hand:
    """Every reviewed input of hand.toml."""

    forms: list[HandForm]
    pages: list[HandPage]
    readings: list[HandReading]


@dataclass(frozen=True)
class DbCatalogue:
    """Sources, forms and definitions by family, and the problems that stop a build."""

    sources: list[dict[str, object]]
    forms: dict[str, list[dict[str, object]]]
    definitions: dict[str, list[dict[str, object]]]
    problems: list[str]
