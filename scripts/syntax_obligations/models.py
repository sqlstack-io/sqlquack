"""Syntax obligation models: the grammar tree and obligation records."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class Node:
    """A grammar token, an optional part `[ ... ]` or a choice `{ a | b }`.

    Optional and choice nodes hold branches; an optional with one branch is a plain option.
    """

    kind: str
    text: str
    branches: tuple[tuple[Node, ...], ...] = ()
    token_kind: str = ""


@dataclass(frozen=True)
class Spec:
    """How to render an obligation's case from its form's grammar trees.

    action: min (no option), max (every option), branch (choice branch `payload` everywhere),
    drop or replace `node` (payload is the replacement), quote (every identifier quoted),
    reserved (`node` as a reserved word), nest (statement or query operands nested).
    `path` names the options that must be present to reach `node`, `chosen` the (choice,
    branch) selections on the way. Nodes are `id()`s of the
    trees `derive_trees` returns.
    """

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
