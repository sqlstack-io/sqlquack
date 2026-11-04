"""Syntax obligation models: the grammar tree and obligation records."""

from __future__ import annotations

from dataclasses import dataclass


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
