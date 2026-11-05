"""The repository cases' queries, each with a schema declaring its input relation."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.format_conformance.constants import (
    COLUMNS_KEY,
    INPUT_RELATION,
    QUERY_KEY,
    QUOTE,
)
from scripts.format_conformance.types import Schema


def case_queries(cases_dir: Path) -> list[tuple[str, str, Schema]]:
    """Every case's id, Snowflake query and schema, in file order."""
    queries: list[tuple[str, str, Schema]] = []
    for path in sorted(cases_dir.rglob("*.toml")):
        document: dict = tomllib.loads(path.read_text())
        for case in document.get("case", []):
            if not {QUERY_KEY, COLUMNS_KEY} <= case.keys():
                continue
            columns: list[list[str]] = [
                [column_name(column["name"]), column["type"]] for column in case[COLUMNS_KEY]
            ]
            queries.append((case["id"], case[QUERY_KEY], {INPUT_RELATION: columns}))
    return queries


def column_name(name: str) -> str:
    """A column name as Snowflake resolves it: quoted names as written, others upper case."""
    quoted: bool = len(name) > 1 and name[0] == name[-1] == QUOTE
    return name[1:-1] if quoted else name.upper()
