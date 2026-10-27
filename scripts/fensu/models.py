"""Rust source tokens and the edits the transformations make to them."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path


@dataclass(frozen=True)
class Token:
    """One lexical token of Rust source: its kind and its byte range in the text."""

    kind: str
    start: int
    end: int
    text: str


@dataclass(frozen=True)
class Edit:
    """Replace the text in `start..end` with `replacement`."""

    start: int
    end: int
    replacement: str


@dataclass(frozen=True)
class Item:
    """A Rust item: its kind and name, and where its text, header and body lie."""

    kind: str
    name: str
    start: int
    end: int
    header_start: int
    keyword_start: int
    visibility: str
    trait_name: str
    body_start: int
    body_end: int
    attributes: tuple[str, ...]


@dataclass(frozen=True)
class UseLeaf:
    """One imported path of a `use` declaration and the name it binds, or a glob."""

    path: tuple[str, ...]
    name: str
    glob: bool


@dataclass(frozen=True)
class Placement:
    """Where a source file's items go: sticky sections, single-item overrides, impl splits."""

    sections: tuple[tuple[str, str], ...]
    overrides: tuple[tuple[str, str], ...] = ()
    methods: tuple[tuple[str, tuple[tuple[str, str], ...]], ...] = ()


@dataclass(frozen=True)
class SourceModule:
    """One module of the crate as written: a file, or an inline `mod` inside one."""

    path: tuple[str, ...]
    file: Path
    text: str
    tokens: list[Token]
    items: list[Item]
    parent: SourceModule | None
    inline: Item | None = None
    children: dict[str, SourceModule] = field(default_factory=dict)
    defs: dict[str, Item] = field(default_factory=dict)
    uses: list[tuple[UseLeaf, Item]] = field(default_factory=list)


@dataclass(frozen=True)
class Definition:
    """A named item where it was written: its module path, name and kind."""

    module: tuple[str, ...]
    name: str
    kind: str


@dataclass(frozen=True)
class External:
    """A name a module imports from outside the crate, by its full path."""

    path: tuple[str, ...]


@dataclass(frozen=True)
class Piece:
    """One item bound for a destination file, from one source module."""

    source: SourceModule
    item: Item
    impl_header: str = ""
    inner_test: bool = False
    impl_item: Item | None = None


@dataclass(frozen=True)
class Destination:
    """A new file and the pieces it holds."""

    file: str
    pieces: list[Piece] = field(default_factory=list)


@dataclass(frozen=True)
class Replacement:
    """Tokens from `start` through `stop` (token indices) replaced by `text`."""

    stop: int
    text: str
