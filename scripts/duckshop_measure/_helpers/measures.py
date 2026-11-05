"""The two measures: a census schema from an environment, and every compiled unit test run."""

from __future__ import annotations

import json
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from scripts.census_functions._helpers.schema_file import compiled_functions
from scripts.census_functions.constants import FUNCTIONS_KEY, RELATIONS_KEY
from scripts.duckshop_measure._helpers.functions import create_macros, python_calls
from scripts.duckshop_measure._helpers.outcomes import unit_test_outcome
from scripts.duckshop_measure._helpers.running import refusal_names
from scripts.duckshop_measure._helpers.schema import census_schema, recorded_schema
from scripts.duckshop_measure._helpers.snowflake import (
    compiled_file,
    converted_name,
    duckdb_error,
    load_inputs,
    model_result,
    orders_rows,
    with_unloadable_input,
)
from scripts.duckshop_measure.constants import (
    JSON_SUFFIX,
    MODEL_PREFIX,
    SQL_SUFFIX,
    TARGETS,
    TESTS_DIR,
    WORKERS,
)
from scripts.duckshop_measure.models import (
    Conversion,
    ModelResult,
    SnowflakeInputs,
    UnitTestOutcome,
)
from scripts.duckshop_measure.types import (
    Comparer,
    Converter,
    Executor,
    ModelOutcome,
    RecordedOutcome,
)


def write_schema(
    *, environment: Path, out: Path, recorded: Path | None = None, functions: Path | None = None
) -> None:
    """Write the census schema of the environment's tables, with `functions`' compiled ones."""
    setup: list[str] = json.loads(environment.read_text())["setup_sql"]
    schema: dict[str, list[list[str]]] = census_schema(setup)
    if recorded is not None:
        schema.update(recorded_schema(recorded))
    written: object = schema
    if functions is not None:
        found, unread = compiled_functions(functions)
        written = {RELATIONS_KEY: schema, FUNCTIONS_KEY: found}
        print(f"duckshop measure: {len(found)} functions written, {unread} unread", file=sys.stderr)
    out.write_text(json.dumps(written, indent=1) + "\n")
    print(f"duckshop measure: {len(schema)} relations written to {out}", file=sys.stderr)


def model_python_calls(
    *, inputs: SnowflakeInputs, sql: Path, refusals: tuple[str, ...]
) -> tuple[str, ...]:
    """The Python functions a model's conversion or refusals name."""
    converted: tuple[str, ...] = (sql.read_text(),) if sql.exists() else ()
    return python_calls(texts=(*converted, *refusals), python=inputs.python_functions)


def unit_test_outcomes(
    *, compiled: Path, convert: Converter, execute: Executor
) -> list[UnitTestOutcome]:
    """Every compiled unit test, converted and run, in path order."""
    tests_root: Path = compiled / TESTS_DIR
    files: list[Path] = sorted(tests_root.rglob(f"*{SQL_SUFFIX}"))
    with ThreadPoolExecutor(WORKERS) as pool:
        conversions: list[Conversion] = list(pool.map(convert, files))
    return [
        unit_test_outcome(
            test=str(path.relative_to(tests_root)), conversion=conversion, execute=execute
        )
        for path, conversion in zip(files, conversions, strict=True)
    ]


def census_refusals(census: Path) -> dict[str, tuple[str, ...]]:
    """Each model's own refusals in a `census --json --names` report, by model name."""
    report: dict[str, dict[str, list[dict[str, object]]]] = json.loads(census.read_text())
    return {str(item["model"]): refusal_names(item["own"]) for item in report["blockers"]["files"]}


def value_divergences(*, inputs: SnowflakeInputs, name: str) -> tuple[str, ...]:
    """The value divergences the conversion written as `name` relies on, from census's guards."""
    if inputs.guards is None:
        return ()
    path: Path = inputs.guards / f"{name}{JSON_SUFFIX}"
    if not path.exists():
        return ()
    used: dict[str, object] = json.loads(path.read_text())["divergences"]
    return tuple(sorted(set(used) & inputs.value_divergences))


def snowflake_results(*, inputs: SnowflakeInputs, run: Comparer) -> list[ModelResult]:
    """Every recorded model: its census refusals, or its run over recorded inputs compared."""
    results: Path = inputs.results
    compiled: Path = inputs.compiled
    converted: Path = inputs.converted
    work: Path = inputs.work
    own: dict[str, tuple[str, ...]] = census_refusals(inputs.census)
    sidecars: list[dict[str, object]] = []
    comparisons: list[dict[str, object]] = []
    errors: dict[str, str | None] = {}
    unloadable: dict[str, dict[str, str]] = {}
    for target in TARGETS:
        entries: list[dict[str, object]] = json.loads(
            (results.parent / f"index-{target}.json").read_text()
        )
        database: Path = work / f"{target}.duckdb"
        refused: dict[str, str] = load_inputs(entries=entries, results=results, database=database)
        failed: dict[str, str] = create_macros(database=database, macros=list(inputs.macros))
        refused.update(
            (name.lower(), why) for name, why in (*inputs.unavailable_functions, *failed.items())
        )
        print(
            f"duckshop measure: {len(refused)} inputs or functions unavailable for {target}",
            file=sys.stderr,
        )
        for entry in entries:
            unloadable[str(entry["key"])] = refused
            if not str(entry["unique_id"]).startswith(MODEL_PREFIX):
                continue
            sidecar: dict[str, object] = json.loads((results / f"{entry['key']}.json").read_text())
            sidecars.append(sidecar)
            name: str = converted_name(
                compiled=compiled, compiled_sql_path=str(sidecar["compiled_sql_path"])
            )
            sql: Path = converted / name
            model_refusals: tuple[str, ...] = own.get(
                Path(str(sidecar["compiled_sql_path"])).stem, ()
            )
            if model_python_calls(inputs=inputs, sql=sql, refusals=model_refusals):
                continue
            if not sql.exists() or value_divergences(inputs=inputs, name=name):
                continue
            if sidecar["outcome"] == RecordedOutcome.ERROR:
                errors[str(entry["key"])] = duckdb_error(sql=sql.read_text(), database=database)
                continue
            if sidecar["outcome"] != RecordedOutcome.ROWS:
                continue
            source: str = compiled_file(
                compiled=compiled, compiled_sql_path=str(sidecar["compiled_sql_path"])
            ).read_text()
            comparisons.append(
                {
                    "id": entry["key"],
                    "recorded": str(results / f"{entry['key']}.json"),
                    "database": str(database),
                    "sql": str(sql),
                    "ordered": orders_rows(source),
                }
            )
    compared: dict[str, dict[str, str]] = run(
        manifest=work / "manifest.json", comparisons=comparisons
    )
    outcomes: list[ModelResult] = []
    for sidecar in sidecars:
        key: str = str(sidecar["key"])
        name = converted_name(
            compiled=compiled, compiled_sql_path=str(sidecar["compiled_sql_path"])
        )
        refusals: tuple[str, ...] | None = None
        declared: tuple[str, ...] = value_divergences(inputs=inputs, name=name)
        if not (converted / name).exists():
            refusals = own.get(Path(str(sidecar["compiled_sql_path"])).stem) or (
                "no census record",
            )
            lenient: Path | None = inputs.lenient_converted
            declared = refusals if lenient is not None and (lenient / name).exists() else ()
        python: tuple[str, ...] = model_python_calls(
            inputs=inputs,
            sql=converted / name,
            refusals=own.get(Path(str(sidecar["compiled_sql_path"])).stem, ()),
        )
        if python:
            outcomes.append(
                ModelResult(
                    model=str(sidecar["model"]),
                    target=str(sidecar["target"]),
                    outcome=ModelOutcome.PYTHON_UDF_NOT_MEASURED,
                    details=python,
                )
            )
            continue
        outcomes.append(
            model_result(
                sidecar=sidecar,
                refusals=refusals,
                compared=with_unloadable_input(
                    compared=compared.get(key), unloadable=unloadable.get(key, {})
                ),
                error=errors.get(key),
                declared=declared,
            )
        )
    return sorted(outcomes, key=lambda outcome: (outcome.target, outcome.model))
