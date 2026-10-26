"""Samples written as Parquet, a stand-in converter, and a measurement run over them."""

from __future__ import annotations

import json
from pathlib import Path

import duckdb

from scripts.guard_measurement.main.measure import run_measure
from scripts.guard_measurement.models import ConvertedProbe

RELATION: str = "SHOP.RAW.ORDERS"

_TAG: str = (
    '(CASE WHEN "GUARD_SAMPLE"."X1" LIKE \'bad%\' THEN '
    "CAST(CAST('json_fidelity: a value DuckDB may read differently' AS INTEGER) AS \"NULL\") "
    'WHEN "GUARD_SAMPLE"."X1" LIKE \'err%\' THEN '
    "CAST(CAST('sqlquack:100035: Timestamp is not recognized' AS INTEGER) AS \"NULL\") "
    'ELSE "GUARD_SAMPLE"."X1" END) IS NULL'
)
_NUMBERS: str = (
    '(CASE WHEN regexp_matches("GUARD_SAMPLE"."X1", \'[eE]|[0-9]{16}\') THEN '
    "CAST(CAST('json_fidelity: JSON with a number, word or comma DuckDB may read "
    'differently from Snowflake\' AS INTEGER) AS "NULL") ELSE "GUARD_SAMPLE"."X1" END) IS NULL'
)


def _emitted(expression: str) -> str:
    """A table macro body in the shape `sqlquack function` emits for a probe."""
    return (
        'TABLE SELECT "__sqlquack_body"."R" FROM (SELECT CAST(p1 AS DECIMAL(1,0)) AS "UNUSED") '
        'AS "__SQLQUACK_PARAMETERS" CROSS JOIN LATERAL (SELECT '
        f'{expression} AS "R" FROM "GUARD_SAMPLE" AS "GUARD_SAMPLE") AS "__sqlquack_body"'
    )


_BODIES: dict[str, str] = {
    "TAG(x1)": _emitted(_TAG),
    "NUMBERS(x1)": _emitted(_NUMBERS),
    "NATIVE(x1)": _emitted(
        _TAG.replace(
            "CAST(CAST('sqlquack:100035: Timestamp is not recognized' AS INTEGER)",
            'CAST(CAST("GUARD_SAMPLE"."X1" AS INTEGER)',
        )
    ),
    "UNSHAPED(x1)": f'TABLE SELECT {_TAG} AS "R" FROM "GUARD_SAMPLE" AS "GUARD_SAMPLE"',
}


_REFUSAL: str = "missing rule: no rule converts fn:UNKNOWN_CALL"


def stand_in_converter(*, body: str, columns: tuple[tuple[str, str], ...]) -> ConvertedProbe | str:
    """The fixed DuckDB body for the probe's expression, or a refusal."""
    expression: str = body.removeprefix("SELECT (").removesuffix(") IS NULL AS r FROM guard_sample")
    converted: dict[str, ConvertedProbe] = {
        name: ConvertedProbe(parameters=("p1",), body=emitted) for name, emitted in _BODIES.items()
    }
    return converted.get(expression, _REFUSAL)


def measure(
    tmp_path: Path,
    values: tuple[str | None, ...],
    expression: str,
    declared_type: str,
    options: tuple[str, ...] = (),
) -> dict[str, object]:
    """Write `values` as column NOTE of the sample, measure one probe and return the report."""
    connection: duckdb.DuckDBPyConnection = duckdb.connect()
    connection.execute("CREATE TABLE notes (NOTE VARCHAR)")
    connection.executemany("INSERT INTO notes VALUES (?)", [[value] for value in values])
    connection.execute(f"COPY notes TO '{tmp_path / (RELATION + '.parquet')}' (FORMAT parquet)")
    (tmp_path / f"{RELATION}.types.json").write_text('{"note": "VARCHAR"}', encoding="utf-8")
    sources: Path = tmp_path / "sources.json"
    sources.write_text(
        json.dumps(
            {
                "probes": [
                    {
                        "divergence": "json_fidelity",
                        "construct": "fn:PARSE_JSON",
                        "relation": RELATION,
                        "expression": expression,
                        "arguments": [["NOTE", declared_type]],
                        "semi_structured": False,
                        "aggregate": False,
                    }
                ]
            }
        ),
        encoding="utf-8",
    )
    out: Path = tmp_path / "report.json"
    exit_code: int = run_measure(
        argv=["--sources", str(sources), "--samples", str(tmp_path), "--out", str(out), *options],
        converter=stand_in_converter,
    )
    assert exit_code == 0
    return json.loads(out.read_text(encoding="utf-8"))
