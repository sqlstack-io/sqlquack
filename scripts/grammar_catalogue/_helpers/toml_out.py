"""Write catalogue tables as TOML arrays of tables, one key per line."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.grammar_catalogue.types import Row


def toml_value(value: object) -> str:
    """A TOML literal for a string, integer, boolean or list of those."""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, list):
        if not value:
            return "[]"
        return "[\n" + "".join(f"  {toml_value(item)},\n" for item in value) + "]"
    return json.dumps(value, ensure_ascii=False)


def row_lines(*, table: str, row: Row) -> list[str]:
    """One `[[table]]` block followed by a blank line."""
    return [f"[[{table}]]", *(f"{key} = {toml_value(value)}" for key, value in row.items()), ""]


def write_tables(*, path: Path, header: str, tables: list[tuple[str, list[Row]]]) -> None:
    """Write a header comment and the given arrays of tables."""
    lines: list[str] = [header.rstrip(), ""]
    for table, rows in tables:
        for row in rows:
            lines += row_lines(table=table, row=row)
    path.write_text("\n".join(lines))
