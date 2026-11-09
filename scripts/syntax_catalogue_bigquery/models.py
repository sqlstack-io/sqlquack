"""BigQuery catalogue models: pinned pages, grammar rules and what the build and audit read."""

from __future__ import annotations

from dataclasses import dataclass, field

from scripts.grammar_catalogue.models import Catalogue, FormGroup, HandForm
from scripts.syntax_catalogue_bigquery.types import Node


@dataclass(frozen=True)
class FetchedPage:
    """One fetched reference page: its path, URL, retrieval time, status, size and hash."""

    path: str
    url: str
    retrieved_utc: str
    status: int | str
    size: int
    sha256: str


@dataclass(frozen=True)
class Located:
    """One syntax block, signature, table row or code span with the heading path above it."""

    heading: tuple[str, ...]
    text: str
    fenced: bool = False
    table: int = 0
    where: str = ""


@dataclass(frozen=True)
class BigQueryPage:
    """One pinned reference page, parsed into what the build reads."""

    path: str
    id: str
    url: str
    title: str
    retrieved_utc: str
    page_sha256: str
    blocks: tuple[Located, ...]
    rows: tuple[Located, ...]
    text: str


@dataclass(frozen=True)
class HandPage:
    """The reviewed disposition of a page that yields no form."""

    source: str
    status: str
    reason: str
    page_sha256: str


@dataclass(frozen=True)
class BlockReview:
    """The reviewed disposition of one syntax block, bound to the block's sha256."""

    source: str
    block_sha256: str
    status: str
    reason: str
    grammar: tuple[str, ...] = ()
    family: str = ""


@dataclass(frozen=True)
class HandInputs:
    """hand.toml: hand forms by page with their family, page and block dispositions."""

    forms: dict[str, list[tuple[str, HandForm]]]
    pages: dict[str, HandPage]
    blocks: dict[tuple[str, str], BlockReview]
    signatures: dict[tuple[str, str], str]


@dataclass(frozen=True)
class PageBuild:
    """What one page contributes: groups and definitions by (family, section base)."""

    groups: dict[tuple[str, str], list[FormGroup]] = field(default_factory=dict)
    definitions: dict[tuple[str, str], list[tuple[str, str]]] = field(default_factory=dict)
    decisions: set[str] = field(default_factory=set)
    syntax: list[str] = field(default_factory=list)
    problems: list[str] = field(default_factory=list)
    restated: set[str] = field(default_factory=set)
    block_names: dict[str, dict[str, tuple[tuple[str, str], int]]] = field(default_factory=dict)


@dataclass(frozen=True)
class BigQueryCatalogue:
    """The built catalogue and every unresolved input."""

    catalogue: Catalogue
    problems: list[str]


@dataclass(frozen=True)
class GrammarRule:
    """One Textmapper nonterminal: its alternatives and the language features its actions test."""

    name: str
    body: Node
    features: tuple[tuple[str, ...], ...]
    line: int


@dataclass(frozen=True)
class CatalogueWords:
    """Each catalogued grammar line's words, its leading keywords, and every grammar word."""

    forms: list[tuple[str, list[str]]]
    every: set[str]
    leads: dict[str, tuple[str, ...]]
    clauses: list[tuple[str, list[str]]]


@dataclass(frozen=True)
class AuditGap:
    """A keyword sequence a grammar rule alternative spells that no catalogued form contains."""

    rule: str
    alternative: int
    keywords: str
    features: tuple[str, ...] = ()


@dataclass(frozen=True)
class AuditResult:
    """What the audit found and how much it checked."""

    gaps: list[AuditGap]
    derived_gaps: int
    missing_keywords: list[str]
    dispatched_words: list[str]
    missing_functions: list[str]
    missing_types: list[str]
    sequences_checked: int
    rules_checked: int
    walked: list[str]
    unwalked: list[str]
    keywords_checked: int
    functions_checked: int
    types_checked: int
    features: dict[str, tuple[str, ...]]
    rejected_alternatives: int
    statements: frozenset[str]
    form_ids: frozenset[str]
    doc_streams: list[tuple[str, list[str]]]
