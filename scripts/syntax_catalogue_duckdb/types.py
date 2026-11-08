"""DuckDB catalogue type aliases."""

from __future__ import annotations

from typing import Any

type Row = dict[str, object]
type Node = tuple[Any, ...]
type Sequences = list[tuple[str, ...]]
type Tree = dict[str, Any]
type Review = dict[str, object]
