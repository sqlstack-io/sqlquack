"""PostgreSQL catalogue models: pinned pages, synopsis blocks, signatures and dispositions."""

from __future__ import annotations

from dataclasses import dataclass, field

from scripts.syntax_catalogue.models import FormGroup


@dataclass(frozen=True)
class Section:
    """One DocBook section: its tag, id and title."""

    tag: str
    id: str
    title: str


@dataclass(frozen=True)
class SynopsisBlock:
    """One `<synopsis>` block with the heading path above it and its grammar text."""

    heading: tuple[str, ...]
    text: str
    sha256: str
    in_synopsis_div: bool


@dataclass(frozen=True)
class Signature:
    """One documented function or operator signature."""

    heading: tuple[str, ...]
    grammar: str
    literals: tuple[str, ...]
    is_function: bool
    raw_sha256: str


@dataclass
class PgPage:
    """One pinned page: a command refentry or a chapter sect1, with what it documents."""

    id: str
    title: str
    family: str
    file: str
    sha256: str
    blocks: list[SynopsisBlock] = field(default_factory=list)
    signatures: list[Signature] = field(default_factory=list)
    is_command: bool = False


@dataclass(frozen=True)
class BlockDisposition:
    """How a synopsis block outside a command's synopsis is read: form, restated or fragment."""

    source: str
    block_sha256: str
    disposition: str
    kind: str
    forms: tuple[str, ...]
    reason: str


@dataclass(frozen=True)
class SignatureDisposition:
    """A documented signature that catalogues no form, with its reviewed reason."""

    source: str
    grammar: str
    disposition: str
    reason: str


@dataclass(frozen=True)
class PgPageResult:
    """One page's form groups by family, its definitions and its extraction decisions."""

    groups: dict[str, list[FormGroup]]
    definitions: list[tuple[str, str]]
    decisions: set[str]
    restates: tuple[str, ...] = ()


@dataclass
class Segment:
    """Part of a synopsis block: statements (no names) or the body of a `where X is:` phrase."""

    names: tuple[str, ...]
    verb: str
    lines: list[str]


@dataclass(frozen=True)
class Definition:
    """A `where X is:` sub-grammar: its name and its alternatives when it lists several."""

    name: str
    alternatives: tuple[str, ...]
    listed: bool


@dataclass(frozen=True)
class Keyword:
    """One kwlist.h keyword: its SQL text, grammar token and category."""

    text: str
    token: str
    category: str


@dataclass(frozen=True)
class Rule:
    """One gram.y rule and its alternatives as token sequences."""

    name: str
    alternatives: tuple[tuple[str, ...], ...]


@dataclass(frozen=True)
class AuditGap:
    """A gram.y alternative whose keyword sequence no catalogued form spells."""

    rule: str
    alternative: int
    keywords: str
    production: str


@dataclass(frozen=True)
class AuditResult:
    """What the omission audit found and how much it checked."""

    gaps: list[AuditGap]
    unused: list[Keyword]
    missing: list[Keyword]
    alternatives_checked: int
    rules_checked: int
    keywords_checked: int
    unwalked: list[str]
