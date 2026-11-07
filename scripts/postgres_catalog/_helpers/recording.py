"""The psql script that types each case through a view, and the fixture written from its output."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.postgres_catalog.constants import (
    TABLE_COLUMNS,
    TYPE_ERROR_MARKER,
    TYPE_FIELD,
    TYPE_MARKER,
)


def recording_script(*, cases: list[tuple[str, str, str]]) -> str:
    """A rolled-back script creating t, then a view per case with its first column's type echoed."""
    columns: str = ", ".join(f"{name} {data_type}" for name, data_type in TABLE_COLUMNS)
    script: list[str] = ["\\set VERBOSITY terse", "BEGIN;", f"CREATE TEMP TABLE t ({columns});"]
    for index, (_, _, sql) in enumerate(cases):
        script += [
            f"SAVEPOINT s{index};",
            f"\\echo {TYPE_ERROR_MARKER}{index}",
            f"CREATE TEMP VIEW v{index} AS {sql};",
            f"SELECT '{TYPE_MARKER}{index}@@' || format_type(atttypid, atttypmod) "
            f"FROM pg_attribute WHERE attrelid = 'v{index}'::regclass AND attnum = 1;",
            f"ROLLBACK TO SAVEPOINT s{index};",
        ]
    script.append("ROLLBACK;")
    return "\n".join(script)


def recorded_cases(*, cases: list[tuple[str, str, str]], output: str) -> list[dict[str, str]]:
    """Each case with the type the server gave it, or the first error it raised."""
    types: dict[int, str] = {}
    errors: dict[int, str] = {}
    current: int = -1
    for line in output.splitlines():
        if line.startswith(TYPE_ERROR_MARKER):
            current = int(line.removeprefix(TYPE_ERROR_MARKER))
        elif line.startswith(TYPE_MARKER):
            index, data_type = line.removeprefix(TYPE_MARKER).split("@@", 1)
            types[int(index)] = data_type
        elif line.startswith(("ERROR:", "psql:")) and current not in errors:
            errors[current] = line.split("ERROR:", 1)[-1].strip()
    records: list[dict[str, str]] = []
    for index, (case_id, category, sql) in enumerate(cases):
        result: dict[str, str] = (
            {TYPE_FIELD: types[index]} if index in types else {"error": errors.get(index, "")}
        )
        records.append({"id": case_id, "category": category, "sql": sql, **result})
    return records


def write_fixture(
    *, records: list[dict[str, str]], version: str, image: str, path: Path, method: str
) -> None:
    """Write the fixture: the engine, its image, the table t and one line per case."""
    header: dict[str, object] = {
        "engine": "postgres",
        "engine_version": version,
        "engine_image_sha256": image,
        "method": method,
        "schema": {
            "tables": [
                {
                    "name": "t",
                    "columns": [{"name": name, "type": kind} for name, kind in TABLE_COLUMNS],
                }
            ]
        },
    }
    body: str = ",\n".join("  " + json.dumps(record) for record in records)
    path.write_text(json.dumps(header, indent=1)[:-2] + f',\n "cases": [\n{body}\n ]\n}}\n')
