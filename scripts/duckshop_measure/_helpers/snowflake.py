"""DuckShop models converted and run on DuckDB over Snowflake's recorded inputs, then compared."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tomllib
from collections.abc import Iterable
from pathlib import Path

import duckdb
import pyarrow.ipc

from scripts.duckshop_measure.constants import (
    CENSUS_TABLE_PREFIX,
    CLOSE_PARENTHESIS,
    COMPILED_PREFIX,
    DUCKDB_TYPES,
    FIXED_TYPE,
    GUARD_RAISE,
    MICROSECOND_SCALE,
    MISSING_TABLE,
    NOT_SCORED_EXECUTIONS,
    OPEN_PARENTHESIS,
    ORDER_BY,
    QUOTE_CHARACTERS,
    REPLAY_SESSION,
    TIMESTAMP_NTZ_TYPE,
    VALUE_FAILURE,
    WORD_JOINER,
)
from scripts.duckshop_measure.exceptions import UncomparedModelError
from scripts.duckshop_measure.models import ModelResult
from scripts.duckshop_measure.types import ModelOutcome, RecordedOutcome


def relation_table(unique_id: str) -> str:
    """The table a census placeholder names for a model, source or seed: `__ref("x")` reads x."""
    return CENSUS_TABLE_PREFIX + unique_id.split(".", 2)[2]


def compiled_file(*, compiled: Path, compiled_sql_path: str) -> Path:
    """The compiled model a sidecar names, under the local `target/compiled`."""
    return compiled / compiled_sql_path.split(COMPILED_PREFIX, 1)[1]


def converted_name(*, compiled: Path, compiled_sql_path: str) -> str:
    """The file census writes a model's conversion to: its compiled path with `/` as `_`."""
    return str(compiled_file(compiled=compiled, compiled_sql_path=compiled_sql_path)).replace(
        "/", "_"
    )


def orders_rows(sql: str) -> bool:
    """Whether `sql` ends in an ORDER BY outside parentheses, quoted text and comments."""
    depth: int = 0
    position: int = 0
    words: list[str] = []
    while position < len(sql):
        character: str = sql[position]
        if sql.startswith("--", position):
            end: int = sql.find("\n", position)
            position = len(sql) if end < 0 else end
            continue
        if sql.startswith("/*", position):
            end = sql.find("*/", position + 2)
            position = len(sql) if end < 0 else end + 2
            continue
        if character in QUOTE_CHARACTERS:
            end = sql.find(character, position + 1)
            position = len(sql) if end < 0 else end + 1
            continue
        if character == OPEN_PARENTHESIS:
            depth += 1
        elif character == CLOSE_PARENTHESIS:
            depth -= 1
        elif depth == 0 and (character.isalpha() or character == WORD_JOINER):
            end = position
            while end < len(sql) and (sql[end].isalnum() or sql[end] == WORD_JOINER):
                end += 1
            words.append(sql[position:end].upper())
            position = end
            continue
        position += 1
    return any(tuple(words[index : index + 2]) == ORDER_BY for index in range(len(words)))


def duckdb_type(column: dict[str, object]) -> str:
    """The DuckDB type the policy pairs with a recorded column's declared Snowflake type."""
    name: str = str(column["type_name"])
    declared: object = column.get("scale")
    scale: int = declared if isinstance(declared, int) else 0
    if name == FIXED_TYPE:
        return f"DECIMAL({column['precision']},{scale})"
    if name == TIMESTAMP_NTZ_TYPE:
        return "TIMESTAMP" if scale <= MICROSECOND_SCALE else "TIMESTAMP_NS"
    return DUCKDB_TYPES[name]


def load_inputs(
    *, entries: Iterable[dict[str, object]], results: Path, database: Path
) -> dict[str, str]:
    """Load each recorded relation typed per the policy; return those DuckDB cannot hold."""
    database.unlink(missing_ok=True)
    connection: duckdb.DuckDBPyConnection = duckdb.connect(str(database))
    unloadable: dict[str, str] = {}
    try:
        for entry in entries:
            arrow: Path = results / f"{entry['key']}.arrow"
            if entry["outcome"] != RecordedOutcome.ROWS or not arrow.exists():
                continue
            table: pyarrow.Table = pyarrow.ipc.open_file(arrow).read_all()
            sidecar: str = (results / f"{entry['key']}.json").read_text()
            columns: list[dict[str, object]] = json.loads(sidecar)["columns"]
            typed: str = ", ".join(
                f'CAST("{column["name"]}" AS {duckdb_type(column)}) AS "{column["name"]}"'
                for column in columns
            )
            connection.register("recorded", table)
            name: str = relation_table(str(entry["unique_id"]))
            try:
                connection.execute(f'CREATE TABLE "{name}" AS SELECT {typed} FROM recorded')
            except (duckdb.ConversionException, duckdb.OutOfRangeException) as error:
                unloadable[name] = f"{type(error).__name__}: {error}"
            connection.unregister("recorded")
    finally:
        connection.close()
    return unloadable


def duckdb_error(*, sql: str, database: Path) -> str | None:
    """DuckDB's error running `sql` over the inputs database, or None when it returns rows."""
    connection: duckdb.DuckDBPyConnection = duckdb.connect()
    try:
        connection.execute(REPLAY_SESSION)
        connection.execute(f"ATTACH '{database}' AS inputs (READ_ONLY); USE inputs;")
        connection.execute(sql).fetchall()
    except duckdb.Error as error:
        return f"{type(error).__name__}: {error}"
    finally:
        connection.close()
    return None


def run_comparisons(
    *,
    binary: Path,
    library: Path,
    policy: Path,
    manifest: Path,
    comparisons: list[dict[str, object]],
) -> dict[str, dict[str, str]]:
    """`sqlquack compare` over `comparisons`, each outcome by id."""
    manifest.write_text(json.dumps(comparisons, indent=1) + "\n")
    environment: dict[str, str] = {**os.environ, "LD_LIBRARY_PATH": str(library)}
    completed: subprocess.CompletedProcess[str] = subprocess.run(
        (str(binary), "compare", "--manifest", str(manifest), "--evidence", str(policy)),
        capture_output=True,
        text=True,
        check=True,
        env=environment,
    )
    return {str(item["id"]): item for item in json.loads(completed.stdout)}


def with_unloadable_input(
    *, compared: dict[str, str] | None, unloadable: dict[str, str]
) -> dict[str, str] | None:
    """`compared`, or input_unloadable when its missing table is an input load refused."""
    detail: str = compared["detail"] if compared else ""
    missing: re.Match[str] | None = MISSING_TABLE.search(detail)
    refusal: str | None = unloadable.get(missing[1]) if missing else None
    if compared is None or missing is None or refusal is None:
        return compared
    return {
        **compared,
        "outcome": ModelOutcome.INPUT_UNLOADABLE,
        "detail": f"{missing[1]}: {refusal}",
    }


def model_result(
    *,
    sidecar: dict[str, object],
    refusals: tuple[str, ...] | None,
    compared: dict[str, str] | None,
    error: str | None,
    declared: tuple[str, ...] = (),
) -> ModelResult:
    """One model's outcome: refused, not scored with why, or its comparison with Snowflake."""
    model: str = str(sidecar["model"])
    target: str = str(sidecar["target"])

    def result(*, outcome: ModelOutcome, details: tuple[str, ...] = ()) -> ModelResult:
        return ModelResult(model=model, target=target, outcome=outcome, details=details)

    if sidecar["outcome"] == RecordedOutcome.ERROR:
        behaviour: str = (
            ModelOutcome.REFUSED if refusals is not None else error or "DuckDB returned rows"
        )
        return result(outcome=ModelOutcome.SNOWFLAKE_ERROR, details=(behaviour,))
    translation: tuple[str, ...] = (
        refusals if refusals is not None else tuple(compared.values()) if compared else ()
    )
    if sidecar["outcome"] == RecordedOutcome.UNRECORDABLE:
        reason: str = str(sidecar.get("unrecordable_reason", ""))
        return result(outcome=ModelOutcome.UNRECORDABLE, details=(reason, *translation))
    if sidecar.get("executed_as") in NOT_SCORED_EXECUTIONS:
        executed: str = str(sidecar["executed_as"])
        return result(outcome=ModelOutcome.INCREMENTAL, details=(executed, *translation))
    if sidecar.get("nondeterministic") is True:
        return result(outcome=ModelOutcome.NONDETERMINISTIC, details=translation)
    if declared:
        return result(outcome=ModelOutcome.DECLARED_DIVERGENCE, details=declared)
    if refusals is not None:
        return result(outcome=ModelOutcome.REFUSED, details=refusals)
    if compared is None:
        raise UncomparedModelError(model)
    outcome: ModelOutcome = ModelOutcome(compared["outcome"])
    detail: str = compared["detail"]
    if outcome == ModelOutcome.DUCKDB_ERROR and MISSING_TABLE.search(detail):
        return result(outcome=ModelOutcome.INPUT_UNRECORDED, details=(detail,))
    guard: re.Match[str] | None = GUARD_RAISE.search(detail)
    if outcome == ModelOutcome.DUCKDB_ERROR and guard is not None:
        return result(outcome=ModelOutcome.GUARD_RAISED, details=(guard[1],))
    return result(outcome=outcome, details=(detail,) if detail else ())


def policy_value_divergences(policy: Path) -> frozenset[str]:
    """The divergences the policy marks `failure = "value"`: they change values, never raise."""
    divergences: dict[str, dict[str, object]] = tomllib.loads(policy.read_text()).get(
        "divergences", {}
    )
    return frozenset(
        name for name, entry in divergences.items() if entry.get("failure") == VALUE_FAILURE
    )
