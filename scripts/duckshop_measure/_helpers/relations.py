"""Census relations from a referee environment's tables and the recorded results' columns."""

from __future__ import annotations

import json
import re
from pathlib import Path

from scripts.duckshop_measure.constants import (
    CENSUS_RELATION_PREFIX,
    CLOSE_PARENTHESIS,
    ENVIRONMENT_PREFIX,
    FIXED_TYPE,
    IDENTIFIER_QUOTE,
    NOT_NULL,
    OPEN_PARENTHESIS,
    SCALED_TYPES,
    SNOWFLAKE_SPELLINGS,
    TABLE_PATTERN,
    TARGETS,
    TOP_LEVEL_SEPARATOR,
)
from scripts.duckshop_measure.exceptions import EnvironmentTableError


def top_level_parts(body: str) -> list[str]:
    """`body` split on commas outside parentheses, so `NUMBER(38,0)` stays one type."""
    parts: list[str] = []
    depth: int = 0
    current: str = ""
    for character in body:
        if character == TOP_LEVEL_SEPARATOR and depth == 0:
            parts.append(current)
            current = ""
            continue
        depth += (character == OPEN_PARENTHESIS) - (character == CLOSE_PARENTHESIS)
        current += character
    parts.append(current)
    return parts


def census_relations(setup_sql: list[str]) -> dict[str, list[list[str]]]:
    """Each `rel__x` table as `__SQLQUACK_RELATION__X`, names upper-cased as Snowflake folds."""
    relations: dict[str, list[list[str]]] = {}
    for statement in setup_sql:
        found: re.Match[str] | None = TABLE_PATTERN.match(statement)
        if found is None or not found.group(1).startswith(ENVIRONMENT_PREFIX):
            raise EnvironmentTableError(f"not a CREATE TEMPORARY TABLE rel__…: {statement[:80]}")
        name: str = found.group(1).removeprefix(ENVIRONMENT_PREFIX)
        columns: list[list[str]] = []
        for part in top_level_parts(found.group(2)):
            column, data_type = NOT_NULL.sub("", part.strip()).split(None, 1)
            columns.append([column.strip(IDENTIFIER_QUOTE).upper(), data_type.strip()])
        relations[f"{CENSUS_RELATION_PREFIX}{name.upper()}"] = columns
    return relations


def snowflake_spelling(column: dict[str, object]) -> str:
    """A recorded column's declared Snowflake type, spelled as a source file declares it."""
    name: str = str(column["type_name"])
    if name == FIXED_TYPE:
        return f"NUMBER({column['precision']},{column['scale']})"
    if name in SCALED_TYPES:
        return f"{name}({column['scale']})"
    return SNOWFLAKE_SPELLINGS.get(name, name)


def recorded_relations(results: Path) -> dict[str, list[list[str]]]:
    """Every recorded source, seed and model with columns, typed as Snowflake declared them."""
    relations: dict[str, list[list[str]]] = {}
    for target in TARGETS:
        for entry in json.loads((results.parent / f"index-{target}.json").read_text()):
            sidecar: dict[str, list[dict[str, object]]] = json.loads(
                (results / f"{entry['key']}.json").read_text()
            )
            relation: str = (
                CENSUS_RELATION_PREFIX + str(entry["unique_id"]).split(".", 2)[2].upper()
            )
            if sidecar.get("columns"):
                relations.setdefault(
                    relation,
                    [
                        [str(column["name"]), snowflake_spelling(column)]
                        for column in sidecar["columns"]
                    ],
                )
    return relations
