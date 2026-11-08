"""SQLite catalogue models: pinned pages, reviewed inputs and what the audit reads."""

from __future__ import annotations

from dataclasses import dataclass, field

from scripts.syntax_catalogue.models import Catalogue, FormGroup, HandForm
from scripts.syntax_catalogue_sqlite.classes.automaton import Automaton


@dataclass(frozen=True)
class Diagram:
    """One syntax diagram: its name, the SHA-256 of its SVG, and its box and annotation labels."""

    name: str
    svg_sha256: str
    labels: tuple[str, ...]
    annotations: tuple[str, ...]


@dataclass(frozen=True)
class SqlitePage:
    """One page of the page set, with its diagrams and documented signatures."""

    path: str
    id: str
    url: str
    title: str
    page_sha256: str
    html: str
    diagrams: tuple[Diagram, ...]


@dataclass(frozen=True)
class Transcription:
    """A diagram read into the catalogue notation by hand, bound to the SVG it reads."""

    name: str
    svg_sha256: str
    grammar: tuple[str, ...]
    labels: tuple[str, ...]
    note: str


@dataclass(frozen=True)
class Override:
    """A documented signature or PRAGMA spelling read by hand, with the reason."""

    source: str
    text: str
    grammar: tuple[str, ...]
    reason: str


@dataclass(frozen=True)
class HandPage:
    """The reviewed status of a page with no diagram, signature or hand form."""

    source: str
    status: str
    reason: str
    page_sha256: str


@dataclass(frozen=True)
class HandInputs:
    """hand.toml: diagram transcriptions, hand forms, overrides and page statuses."""

    diagrams: dict[str, Transcription]
    forms: dict[str, list[tuple[str, HandForm]]]
    overrides: dict[tuple[str, str], Override]
    pages: dict[str, HandPage]


@dataclass(frozen=True)
class Signature:
    """One documented call signature or PRAGMA spelling, as the page writes it."""

    name: str
    text: str
    anchor: str


@dataclass(frozen=True)
class PageBuild:
    """What one page contributes: (family, id base, group) forms, decisions and problems."""

    forms: list[tuple[str, str, FormGroup]] = field(default_factory=list)
    restates: dict[str, set[str]] = field(default_factory=dict)
    decisions: set[str] = field(default_factory=set)
    syntax: list[str] = field(default_factory=list)
    problems: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class SqliteCatalogue:
    """The built catalogue and every unresolved input."""

    catalogue: Catalogue
    problems: list[str]


@dataclass(frozen=True)
class Production:
    """One alternative of a parse.y rule: its right-hand symbols in order."""

    rule: str
    symbols: tuple[str, ...]
    line: int


@dataclass(frozen=True)
class Grammar:
    """parse.y as the pinned build compiles it: productions by rule, tokens and fallbacks."""

    rules: dict[str, list[Production]]
    tokens: frozenset[str]
    start: str
    token_classes: dict[str, tuple[str, ...]]
    fallback: frozenset[str]
    wildcard: frozenset[str]


@dataclass(frozen=True)
class Keyword:
    """One row of mkkeywordhash.c's keyword table: the word and the token it scans as."""

    word: str
    token: str


@dataclass(frozen=True)
class CatalogueWords:
    """Each catalogued grammar line's words, its leading keywords, and every grammar word."""

    forms: list[tuple[str, list[str]]]
    every: set[str]
    leads: dict[str, tuple[str, ...]]
    clauses: list[tuple[str, list[str]]]
    automata: list[tuple[str, Automaton, int]]


@dataclass(frozen=True)
class AuditGap:
    """A keyword sequence a parse.y production spells that no catalogued form contains."""

    rule: str
    alternative: int
    keywords: str


@dataclass(frozen=True)
class EngineName:
    """A keyword, function, PRAGMA or type name, with its kind."""

    name: str
    kind: str


@dataclass(frozen=True)
class AuditResult:
    """What the audit found and how much it checked."""

    gaps: list[AuditGap]
    derived_gaps: int
    missing_keywords: list[EngineName]
    unused_keywords: list[EngineName]
    engine_keywords: list[str]
    missing_functions: list[EngineName]
    missing_pragmas: list[EngineName]
    missing_types: list[EngineName]
    sequences_checked: int
    rules_checked: int
    productions_checked: int
    productions_read: int
    productions_generated: int
    walked: list[str]
    unwalked: list[str]
    functions_checked: int
    keywords_checked: int
    pragmas_checked: int
    types_checked: int
