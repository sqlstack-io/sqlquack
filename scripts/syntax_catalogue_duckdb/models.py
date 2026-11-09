"""DuckDB catalogue models: pinned pages and what the build reads from them."""

from __future__ import annotations

from dataclasses import dataclass, field

from scripts.grammar_catalogue.models import Catalogue, FormGroup, HandForm
from scripts.syntax_catalogue_duckdb.types import Node


@dataclass(frozen=True)
class Located:
    """One diagram id, signature, table row or code block with the heading path above it."""

    heading: tuple[str, ...]
    text: str
    depth: int = 0
    owner: str = ""
    table: int = 0


@dataclass(frozen=True)
class DuckDbPage:
    """One in-scope page of the pinned docs tree."""

    path: str
    id: str
    url: str
    title: str
    railroad: str
    page_sha256: str
    diagrams: tuple[Located, ...]
    signatures: tuple[Located, ...]
    rows: tuple[Located, ...]
    blocks: tuple[Located, ...]


@dataclass(frozen=True)
class HandPage:
    """The reviewed disposition of a page with no diagram, signature, type table or hand form."""

    source: str
    status: str
    reason: str
    page_sha256: str


@dataclass(frozen=True)
class HandInputs:
    """hand.toml: hand forms by page with their family, page dispositions, signature exceptions."""

    forms: dict[str, list[tuple[str, HandForm]]]
    pages: dict[str, HandPage]
    signatures: dict[tuple[str, str], str]


@dataclass(frozen=True)
class PageBuild:
    """What one page contributes: form groups by family, definitions, decisions and problems."""

    groups: dict[str, list[FormGroup]] = field(default_factory=dict)
    definitions: list[tuple[str, str]] = field(default_factory=list)
    decisions: set[str] = field(default_factory=set)
    syntax: list[str] = field(default_factory=list)
    problems: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class DuckDbCatalogue:
    """The built catalogue and every unresolved input."""

    catalogue: Catalogue
    problems: list[str]


@dataclass(frozen=True)
class PegRule:
    """One rule of the PEG grammar: a macro when it has parameters."""

    name: str
    parameters: tuple[str, ...]
    body: Node
    file: str


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


@dataclass(frozen=True)
class EngineName:
    """A function, keyword or type the pinned engine reports, with its kind."""

    name: str
    kind: str


@dataclass(frozen=True)
class AuditResult:
    """What the audit found and how much it checked."""

    gaps: list[AuditGap]
    derived_gaps: int
    missing_keywords: list[EngineName]
    unused_keywords: list[EngineName]
    missing_functions: list[EngineName]
    missing_types: list[EngineName]
    sequences_checked: int
    rules_checked: int
    walked: list[str]
    unwalked: list[str]
    functions_checked: int
    keywords_checked: int
    types_checked: int
