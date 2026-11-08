"""The sizes file `sqlquack verify --sql-sizes` writes, and the committed baseline."""

from __future__ import annotations

from typing import TypedDict


class SizesFile(TypedDict):
    """Each converted case's DuckDB SQL size in bytes, by `<file>#<id>`."""

    format: str
    cases: dict[str, int]
