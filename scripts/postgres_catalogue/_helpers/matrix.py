"""The expression matrix whose result types the 18.6 server records: one case per expression."""

from __future__ import annotations

import json
from itertools import product
from pathlib import Path

from scripts.postgres_catalogue.constants import (
    ACCEPTED,
    AGGREGATE_INPUTS,
    AGGREGATES,
    ARITHMETIC,
    CAST_TARGETS,
    COMPARISONS,
    DATETIME,
    LITERALS,
    NILADIC,
    NUMERIC_COLUMNS,
    QUERY_HEADS,
    SCALARS,
    SHAPES,
    TABLE_COLUMNS,
    TEXT_COLUMNS,
    UNIFY,
)


def _expression_case(*, category: str, expression: str) -> tuple[str, str, str]:
    """One case selecting `expression` from the table t."""
    return (f"{category}: {expression}", category, f"SELECT {expression} AS v FROM t")


def expression_cases() -> list[tuple[str, str, str]]:
    """Every case as (id, category, sql), each typed by its first output."""
    cases: list[tuple[str, str, str]] = []

    for column, _ in TABLE_COLUMNS:
        cases.append(_expression_case(category="column", expression=column))
    for literal in LITERALS:
        cases.append(_expression_case(category="literal", expression=literal))
    for operator, left, right in product(ARITHMETIC, NUMERIC_COLUMNS, NUMERIC_COLUMNS):
        cases.append(
            _expression_case(category="arithmetic", expression=f"{left} {operator} {right}")
        )
    for operator, left, right in product(ARITHMETIC, LITERALS[:4], LITERALS[:4]):
        cases.append(
            _expression_case(category="arithmetic_literal", expression=f"{left} {operator} {right}")
        )
    for operator, column, literal in product(ARITHMETIC, NUMERIC_COLUMNS, ("2", "1.5")):
        cases.append(
            _expression_case(
                category="arithmetic_literal", expression=f"{column} {operator} {literal}"
            )
        )
    for column in NUMERIC_COLUMNS:
        cases.append(_expression_case(category="arithmetic_literal", expression=f"-{column}"))
    for left, right in product((*TEXT_COLUMNS, "'a'"), (*TEXT_COLUMNS, "'b'", "c_integer")):
        cases.append(_expression_case(category="concat", expression=f"{left} || {right}"))
    for expression in COMPARISONS:
        cases.append(_expression_case(category="comparison", expression=expression))
    for function, column in product(AGGREGATES, AGGREGATE_INPUTS):
        cases.append(_expression_case(category="aggregate", expression=f"{function}({column})"))
    for left, right in product(UNIFY, UNIFY):
        cases.append(
            _expression_case(
                category="conditional",
                expression=f"CASE WHEN c_boolean THEN {left} ELSE {right} END",
            )
        )
        cases.append(
            _expression_case(category="conditional", expression=f"coalesce({left}, {right})")
        )
        cases.append(
            _expression_case(category="conditional", expression=f"greatest({left}, {right})")
        )
        cases.append(
            _expression_case(category="conditional", expression=f"nullif({left}, {right})")
        )
        cases.append(
            (
                f"set_operation: {left} UNION ALL {right}",
                "set_operation",
                f"SELECT {left} AS v FROM t UNION ALL SELECT {right} AS v FROM t",
            )
        )
    for expression in DATETIME:
        cases.append(_expression_case(category="datetime", expression=expression))
    for source, target in CAST_TARGETS:
        cases.append(_expression_case(category="cast", expression=f"CAST({source} AS {target})"))
        cases.append(_expression_case(category="cast", expression=f"{source}::{target}"))
    for expression in SCALARS:
        cases.append(_expression_case(category="scalar", expression=expression))
    for name in NILADIC:
        cases.append(_expression_case(category="niladic", expression=name))
    for name, sql in SHAPES:
        cases.append((f"shape: {name}", "shape", sql))
    return cases


def syntax_cases(directory: Path) -> list[tuple[str, str, str]]:
    """Each accepted case of the syntax corpus that reads like a query, as (id, category, sql)."""
    cases: list[tuple[str, str, str]] = []
    for path in sorted(directory.glob("*.jsonl")):
        for line in path.read_text().splitlines():
            case: dict[str, object] = json.loads(line)
            sql: str = str(case["sql"])
            words: list[str] = sql.lstrip().lstrip("(").split(None, 1)
            if case["expect"] == ACCEPTED and words and words[0].upper() in QUERY_HEADS:
                category: str = f"{path.stem}:{case.get('stage') or 'executed'}"
                cases.append((str(case["case_id"]), category, sql))
    return cases
