"""Typed reads of decoded JSON: objects, arrays and nested fields of replies, cases and records."""

from __future__ import annotations

from scripts.bigquery_referee.exceptions import RefereeError
from scripts.bigquery_referee.types import Row


def row_of(value: object) -> Row:
    """A JSON object; anything else as an empty one."""
    return value if isinstance(value, dict) else {}


def items_of(value: object) -> list[object]:
    """A JSON array; anything else as an empty one."""
    return value if isinstance(value, list) else []


def rows_of(value: object) -> list[Row]:
    """The objects of a JSON array, in order; anything else as none."""
    return [item for item in items_of(value) if isinstance(item, dict)]


def field(*, value: object, path: tuple[str, ...]) -> object:
    """The value at a path of keys through nested JSON objects, refused when one is missing."""
    found: object = value
    for key in path:
        if not isinstance(found, dict) or key not in found:
            raise RefereeError(f"no {key!r} on the path {'.'.join(path)}")
        found = found[key]
    return found


def count_of(value: object) -> int:
    """A JSON count, which the API sends as a decimal string; absent or empty as 0."""
    if isinstance(value, bool) or not isinstance(value, int | str):
        return 0
    return int(value) if value else 0
