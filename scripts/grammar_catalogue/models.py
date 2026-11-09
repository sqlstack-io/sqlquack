"""Syntax catalogue models: parsed pages, productions and form groups."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class Block:
    """One fenced sqlsyntax block with the heading path above it."""

    heading: tuple[str, ...]
    text: str


@dataclass(frozen=True)
class Page:
    """A documentation page's title, syntax blocks and syntax headings that have no block."""

    title: str
    blocks: tuple[Block, ...]
    syntax_sections_without_block: tuple[str, ...]


@dataclass(frozen=True)
class Production:
    """A statement, definition or fragment read from a syntax block."""

    kind: str
    grammar: str
    heading: tuple[str, ...]
    name: str = ""


@dataclass(frozen=True)
class FormGroup:
    """The grammars that become one catalogue form, with the label and heading that name it."""

    kind: str
    label: str
    grammars: tuple[str, ...]
    heading: tuple[str, ...]


@dataclass(frozen=True)
class HandForm:
    """A form catalogued by hand from a page whose syntax is not in a sqlsyntax block."""

    source: str
    anchor: str
    kind: str
    label: str
    grammars: tuple[str, ...]
    basis: str
    page_sha256: str


@dataclass(frozen=True)
class PageSource:
    """One fetched page: its cache path, URL, retrieval time, hash and format."""

    path: str
    url: str
    retrieved_utc: str
    sha256: str
    page_format: str


@dataclass(frozen=True)
class Existing:
    """Form ids already in the catalogue, by id, by (source, grammar hash) and by source."""

    forms: dict[str, dict[str, object]]
    by_key: dict[tuple[str, str], str]
    by_source: dict[str, list[str]]
    ids: set[str]


@dataclass(frozen=True)
class Catalogue:
    """Sources, forms by family and definitions by family for every cached page."""

    sources: list[dict[str, object]]
    forms: dict[str, list[dict[str, object]]]
    definitions: dict[str, list[dict[str, object]]]
    folds: dict[str, tuple[tuple[str, ...], str]] = field(default_factory=dict)


@dataclass(frozen=True)
class PageResult:
    """One page's form groups, sub-grammar definitions and extraction decisions."""

    groups: list[FormGroup]
    definitions: list[tuple[str, str]]
    decisions: set[str]


@dataclass(frozen=True)
class PageContext:
    """What naming a page's forms needs: id base, family, source id and sub-grammar names."""

    base: str
    family: str
    source: str
    definitions: tuple[str, ...]
