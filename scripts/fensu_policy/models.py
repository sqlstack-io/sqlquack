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
