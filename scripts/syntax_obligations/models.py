"""Syntax obligation models: the grammar tree, obligation records and case rendering."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class Node:
    """A grammar token, or an optional `[ ... ]` or choice `{ a | b }` holding branches."""

    kind: str
    text: str
    branches: tuple[tuple[Node, ...], ...] = ()
    token_kind: str = ""


@dataclass(frozen=True)
class Spec:
    """A case recipe: an action on `node` (an `id()`) reached via `path` options and `chosen`."""

    action: str
    production: int = 0
    node: int = 0
    path: frozenset[int] = frozenset()
    payload: str = ""
    chosen: tuple[tuple[int, int], ...] = ()
    expect: str = "accept"
    stage: str = ""


@dataclass(frozen=True)
class Draft:
    """An obligation before its id is assigned; pair holds the targets of two option drafts."""

    form: str
    kind: str
    target: str
    note: str
    anchor: str = ""
    depths: tuple[int, ...] = ()
    pair: tuple[str, str] | tuple[()] = ()
    required: bool = True
    spec: Spec | None = field(default=None, compare=False)
    arity: bool = False


@dataclass(frozen=True)
class FormStyle:
    """How a family wraps, fills, declares and opens what one form names to make a submission."""

    prefix: str
    suffix: str
    fills: dict[str, str] = field(default_factory=dict)
    clauses: dict[str, str] = field(default_factory=dict)
    uses: dict[str, str] = field(default_factory=dict)
    opens: str = ""
    elisions: dict[str, str] = field(default_factory=dict)
    literal_fills: dict[str, str] = field(default_factory=dict)
    default_label: str = "l0"
    terminate: bool = True
    embedded: bool = False
    observe_empty_properties: bool = False


@dataclass(frozen=True)
class Rendered:
    """A submission: accept (no syntax error) or reject (a syntax error), and an expected stage."""

    sql: str
    expect: str
    stage: str = ""
