"""BigQuery catalogue type aliases."""

from __future__ import annotations

type Row = dict[str, object]
type Node = tuple[object, ...]
type Sequences = list[tuple[str, ...]]
type Review = dict[str, object]
type TmToken = tuple[str, str, int]
type Nfa = list[list[tuple[str, int]]]
