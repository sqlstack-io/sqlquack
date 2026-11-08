"""JSON values read from a reply or a journal, typed as the referee reads them."""

from __future__ import annotations

from typing import cast

from scripts.databricks_referee.types import Row, Snapshot


def row_of(value: object) -> Row:
    """A copy of a JSON object."""
    return dict(cast(Row, value))


def rows_of(value: object) -> list[Row]:
    """A copy of a JSON array of objects."""
    return list(cast(list[Row], value))


def texts_of(value: object) -> list[str]:
    """A copy of a JSON array of strings."""
    return list(cast(list[str], value))


def data_array(reply: Row) -> list[list[str]]:
    """The rows of a statement's inline JSON result, or none."""
    return cast(list[list[str]], row_of(reply.get("result") or {}).get("data_array", []))


def snapshot_of(value: object) -> Snapshot:
    """The account state a journal's baseline holds."""
    return cast(Snapshot, value)


def count_of(value: object) -> int:
    """A JSON number, or its text, as an integer."""
    return int(cast(int, value))
