"""Value types the layout rules compute before reporting faults."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class CrateName:
    """A product crate name read as sqlquack-<role>[-<dialect>]."""

    role: str
    dialects: tuple[str, ...]


@dataclass(frozen=True)
class CrateDependencies:
    """A crate's name, folder and the names of the workspace crates it uses at runtime."""

    name: str
    directory: str
    runtime_dependencies: tuple[str, ...]


@dataclass(frozen=True)
class SourceLine:
    """One line of a source file that breaks a rule, with the text the fault reports."""

    line: int
    text: str


@dataclass(frozen=True)
class CrateFeatures:
    """The features a crate manifest declares, and the dependencies it marks optional."""

    features: dict[str, tuple[str, ...]]
    optional_dependencies: frozenset[str]


@dataclass(frozen=True)
class DialectVocabulary:
    """The ways Rust code in one crate can name a dialect, each mapped to that dialect."""

    dialects: tuple[str, ...]
    variants: dict[str, str]
    folders: frozenset[str]
    crates: dict[str, str]
