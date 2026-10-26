"""Converted probes run over a sample in DuckDB: raising rows counted, example tuples kept."""

from __future__ import annotations

from pathlib import Path

import duckdb

from scripts.guard_measurement._helpers.counting import (
    hooked_raises,
    probe_expression,
    raising_tuples,
)
from scripts.guard_measurement.constants import (
    EXAMPLE_MODULUS,
    EXAMPLE_SPREAD,
    LEAF_SIZE,
    PROBE_FUNCTION,
    SAMPLE_RELATION,
    SEMI_STRUCTURED_TYPES,
)
from scripts.guard_measurement.models import ConvertedProbe, Evaluation, Found

_STRUCTURAL_ERRORS: tuple[type[Exception], ...] = (
    duckdb.BinderException,
    duckdb.ParserException,
    duckdb.CatalogException,
    duckdb.NotImplementedException,
)


def load_sample(*, connection: duckdb.DuckDBPyConnection, parquet: Path) -> int:
    """The Parquet file as table `sample_rows`, each row numbered `__row`; its row count."""
    connection.execute(
        "CREATE OR REPLACE TABLE sample_rows AS "
        "SELECT row_number() OVER () - 1 AS __row, * FROM read_parquet(?)",
        [str(parquet)],
    )
    return _count(connection=connection, relation="sample_rows")


def evaluate(
    *,
    connection: duckdb.DuckDBPyConnection,
    converted: ConvertedProbe,
    columns: tuple[tuple[str, str], ...],
    aggregate: bool,
    limits: tuple[int, int],
) -> Evaluation:
    """Raising tuples and messages from one hooked pass, else a TRY count with capped examples."""
    examples, budget = limits
    _define_view(connection=connection, columns=columns, source="sample_rows", named=True)
    connection.execute(
        f"CREATE OR REPLACE MACRO {PROBE_FUNCTION}({', '.join(converted.parameters)}) AS "
        f"{converted.body}"
    )
    if aggregate:
        return _aggregate_evaluation(connection=connection)
    selected: str = ", ".join(
        f'"{name}" AS X{index}' for index, (name, _) in enumerate(columns, start=1)
    )
    connection.execute(
        f"CREATE OR REPLACE TABLE tuples AS SELECT row_number() OVER () - 1 AS __idx, "
        f"{selected} FROM sample_rows GROUP BY ALL"
    )
    expression: str | None = probe_expression(body=converted.body)
    if expression is None:
        return _bisected(connection=connection, columns=columns, budget=budget)
    projected: str = _projection(columns=columns)
    messages: dict[int, str] | None = hooked_raises(
        connection=connection, expression=expression, projected=projected
    )
    if messages is not None:
        found: tuple[Found, ...] = _found_all(
            connection=connection, columns=columns, messages=messages
        )
        return Evaluation(
            raising_rows=frozenset().union(*(each.row_ids for each in found)),
            found=found,
            exact=True,
            reasons_exact=True,
        )
    raising: list[int] | None = raising_tuples(
        connection=connection, expression=expression, projected=projected
    )
    if raising is None:
        return _bisected(connection=connection, columns=columns, budget=budget)
    chosen: list[int] = sorted(raising, key=_spread)[:examples]
    examined: list[Found] = []
    for index in chosen:
        _define_view(
            connection=connection, columns=columns, source=_tuples_between(low=index, high=index)
        )
        message: str | None = _raise_of(connection=connection)
        examined.extend(
            []
            if message is None
            else [_found(connection=connection, columns=columns, raised=(index, message))]
        )
    return Evaluation(
        raising_rows=_rows_of(connection=connection, columns=columns, tuples=raising),
        found=tuple(examined),
        exact=True,
        reasons_exact=len(chosen) == len(raising),
    )


def _aggregate_evaluation(*, connection: duckdb.DuckDBPyConnection) -> Evaluation:
    """An aggregate raises over the whole sample or not at all."""
    message: str | None = _raise_of(connection=connection)
    every_row: frozenset[int] = frozenset(
        range(_count(connection=connection, relation="sample_rows"))
    )
    found: tuple[Found, ...] = (
        () if message is None else (Found(message=message, values=(), row_ids=every_row),)
    )
    return Evaluation(
        raising_rows=every_row if found else frozenset(),
        found=found,
        exact=True,
        reasons_exact=True,
    )


def _bisected(
    *, connection: duckdb.DuckDBPyConnection, columns: tuple[tuple[str, str], ...], budget: int
) -> Evaluation:
    """Raising tuples found by bisection within `budget` queries; a lower bound when it runs out."""
    last: int = _count(connection=connection, relation="tuples") - 1
    raising, _, exact = _bisect(
        connection=connection, columns=columns, span=(0, last), budget=budget
    )
    found: tuple[Found, ...] = tuple(
        _found(connection=connection, columns=columns, raised=pair) for pair in raising
    )
    return Evaluation(
        raising_rows=frozenset().union(*(each.row_ids for each in found)),
        found=found,
        exact=exact,
        reasons_exact=exact,
    )


def _found_all(
    *,
    connection: duckdb.DuckDBPyConnection,
    columns: tuple[tuple[str, str], ...],
    messages: dict[int, str],
) -> tuple[Found, ...]:
    """Every raising tuple with its message, values (for reasons only) and sample rows."""
    connection.execute(
        "CREATE OR REPLACE TABLE raising AS SELECT unnest(?::BIGINT[]) AS __idx",
        [list(messages)],
    )
    matched: str = " AND ".join(
        f's."{name}" IS NOT DISTINCT FROM t.X{position}'
        for position, (name, _) in enumerate(columns, start=1)
    )
    held: list[tuple[object, ...]] = connection.execute(
        f"SELECT t.*, list(s.__row) AS __rows FROM tuples AS t JOIN sample_rows AS s ON {matched} "
        "SEMI JOIN raising AS r ON r.__idx = t.__idx GROUP BY ALL"
    ).fetchall()
    return tuple(_held_found(row=row, messages=messages) for row in held)


def _held_found(*, row: tuple[object, ...], messages: dict[int, str]) -> Found:
    """One fetched raising tuple: its number, values and the list of its sample rows."""
    rows: object = row[-1]
    listed: list[object] = rows if isinstance(rows, list) else []
    return Found(
        message=messages[int(str(row[0]))],
        values=tuple(None if value is None else str(value) for value in row[1:-1]),
        row_ids=frozenset(int(str(each)) for each in listed),
    )


def _spread(index: int) -> int:
    """A deterministic order scattering tuple numbers, so examples are not the first tuples."""
    return (index * EXAMPLE_SPREAD) % EXAMPLE_MODULUS


def _rows_of(
    *,
    connection: duckdb.DuckDBPyConnection,
    columns: tuple[tuple[str, str], ...],
    tuples: list[int],
) -> frozenset[int]:
    """The sample rows holding any of `tuples`."""
    connection.execute(
        "CREATE OR REPLACE TABLE raising AS SELECT unnest(?::BIGINT[]) AS __idx", [tuples]
    )
    matched: str = " AND ".join(
        f's."{name}" IS NOT DISTINCT FROM t.X{position}'
        for position, (name, _) in enumerate(columns, start=1)
    )
    rows: list[tuple[int]] = connection.execute(
        f"SELECT s.__row FROM sample_rows AS s JOIN tuples AS t ON {matched} "
        "SEMI JOIN raising AS r ON r.__idx = t.__idx"
    ).fetchall()
    return frozenset(row[0] for row in rows)


def _projection(*, columns: tuple[tuple[str, str], ...]) -> str:
    """The tuple columns as `GUARD_SAMPLE` holds them, semi-structured ones as JSON."""
    return ", ".join(
        f"CAST(X{index} AS JSON) AS X{index}"
        if data_type.upper().startswith(SEMI_STRUCTURED_TYPES)
        else f"X{index}"
        for index, (_, data_type) in enumerate(columns, start=1)
    )


def _bisect(
    *,
    connection: duckdb.DuckDBPyConnection,
    columns: tuple[tuple[str, str], ...],
    span: tuple[int, int],
    budget: int,
) -> tuple[list[tuple[int, str]], int, bool]:
    """Raising tuple numbers with messages in `span`, queries used, and whether all were checked."""
    low, high = span
    if low > high:
        return [], 0, True
    if budget <= 0:
        return [], 0, False
    _define_view(connection=connection, columns=columns, source=_tuples_between(low=low, high=high))
    if _raise_of(connection=connection) is None:
        return [], 1, True
    if high - low < LEAF_SIZE:
        return _each(connection=connection, columns=columns, span=span), 1 + high - low + 1, True
    middle: int = (low + high) // 2
    first, used_first, exact_first = _bisect(
        connection=connection, columns=columns, span=(low, middle), budget=budget - 1
    )
    second, used_second, exact_second = _bisect(
        connection=connection,
        columns=columns,
        span=(middle + 1, high),
        budget=budget - 1 - used_first,
    )
    return first + second, 1 + used_first + used_second, exact_first and exact_second


def _each(
    *,
    connection: duckdb.DuckDBPyConnection,
    columns: tuple[tuple[str, str], ...],
    span: tuple[int, int],
) -> list[tuple[int, str]]:
    """Each raising tuple number in `span` with its message, one tuple at a time."""
    raising: list[tuple[int, str]] = []
    for index in range(span[0], span[1] + 1):
        _define_view(
            connection=connection, columns=columns, source=_tuples_between(low=index, high=index)
        )
        message: str | None = _raise_of(connection=connection)
        raising.extend([(index, message)] if message is not None else [])
    return raising


def _found(
    *,
    connection: duckdb.DuckDBPyConnection,
    columns: tuple[tuple[str, str], ...],
    raised: tuple[int, str],
) -> Found:
    """A raising tuple's values, as text for reasons only, and the sample rows holding it."""
    index, message = raised
    fetched: tuple[object, ...] | None = connection.execute(
        "SELECT * EXCLUDE (__idx) FROM tuples WHERE __idx = ?", [index]
    ).fetchone()
    matched: str = " AND ".join(
        f's."{name}" IS NOT DISTINCT FROM t.X{position}'
        for position, (name, _) in enumerate(columns, start=1)
    )
    rows: list[tuple[int]] = connection.execute(
        f"SELECT s.__row FROM sample_rows AS s JOIN tuples AS t ON {matched} WHERE t.__idx = ?",
        [index],
    ).fetchall()
    return Found(
        message=message,
        values=tuple(None if value is None else str(value) for value in fetched or ()),
        row_ids=frozenset(row[0] for row in rows),
    )


def _define_view(
    *,
    connection: duckdb.DuckDBPyConnection,
    columns: tuple[tuple[str, str], ...],
    source: str,
    named: bool = False,
) -> None:
    """`GUARD_SAMPLE` as the probe's columns of `source`, semi-structured ones as JSON."""
    projected: list[str] = []
    for index, (name, data_type) in enumerate(columns, start=1):
        column: str = f'"{name}"' if named else f"X{index}"
        semi: bool = data_type.upper().startswith(SEMI_STRUCTURED_TYPES)
        projected.append(f"CAST({column} AS JSON) AS X{index}" if semi else f"{column} AS X{index}")
    connection.execute(
        f"CREATE OR REPLACE VIEW {SAMPLE_RELATION} AS SELECT {', '.join(projected)} FROM {source}"
    )


def _tuples_between(*, low: int, high: int) -> str:
    """The value tuples numbered `low` to `high`."""
    return f"(SELECT * FROM tuples WHERE __idx BETWEEN {low} AND {high})"


def _raise_of(*, connection: duckdb.DuckDBPyConnection) -> str | None:
    """The message of the probe's raise over the current view, or None when it runs cleanly."""
    try:
        connection.execute(
            f"SELECT sum(CASE WHEN r THEN 1 ELSE 2 END) FROM {PROBE_FUNCTION}(0)"
        ).fetchall()
    except _STRUCTURAL_ERRORS:
        raise
    except duckdb.Error as error:
        return str(error)
    return None


def _count(*, connection: duckdb.DuckDBPyConnection, relation: str) -> int:
    """The number of rows in `relation`."""
    counted: tuple[int] | None = connection.execute(f"SELECT count(*) FROM {relation}").fetchone()
    return 0 if counted is None else counted[0]
