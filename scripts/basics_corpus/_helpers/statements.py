"""The basic statements, each written once in plain upper-case keywords."""

from __future__ import annotations

from collections.abc import Iterator

from scripts.basics_corpus.constants import (
    AGGREGATES,
    CAST_SUBJECTS,
    CASTS,
    COLUMNS,
    DERIVED,
    FIXED,
    FUNCTIONS,
    JOINS,
    LITERALS,
    OPERANDS_LEFT,
    OPERANDS_RIGHT,
    OPERATORS,
    PREDICATES,
    TRY_CAST,
    TRY_CAST_ONLY_SUBJECT,
    UNION_PAIRS,
    UNIONS,
    WINDOWS,
)
from scripts.basics_corpus.models import Basic


def basics() -> Iterator[Basic]:
    """Every basic statement: select items with and without an alias, then whole-query shapes."""
    for shape, expression in _expressions():
        reads: bool = any(column in expression for column in COLUMNS)
        source: str = " FROM orders" if reads else ""
        yield Basic(f"select_{shape}", f"SELECT {expression} AS v{source}", True)
        yield Basic(f"select_{shape}", f"SELECT {expression}{source}", False)
        if not reads:
            yield Basic(f"select_{shape}_from", f"SELECT {expression} AS v FROM orders", True)
            yield Basic(f"select_{shape}_from", f"SELECT {expression} FROM orders", False)
    for aggregate in AGGREGATES:
        yield Basic("aggregate", f"SELECT {aggregate} AS n FROM orders", True)
        yield Basic("aggregate", f"SELECT {aggregate} FROM orders", False)
        yield Basic(
            "group_by", f"SELECT status, {aggregate} AS n FROM orders GROUP BY status", True
        )
        yield Basic("group_by", f"SELECT status, {aggregate} FROM orders GROUP BY status", False)
        yield Basic(
            "group_by", f"SELECT status, {aggregate} AS n FROM orders GROUP BY 1 ORDER BY 1", True
        )
    for predicate in PREDICATES:
        yield Basic("where", f"SELECT order_id FROM orders WHERE {predicate}", True)
    for column in COLUMNS:
        yield Basic(
            "order_limit",
            f"SELECT order_id, {column} FROM orders ORDER BY {column} DESC LIMIT 3",
            True,
        )
        yield Basic(
            "order_limit",
            f"SELECT order_id FROM orders ORDER BY {column} NULLS LAST LIMIT 2 OFFSET 1",
            True,
        )
    for expression in DERIVED:
        yield Basic(
            "cte", f"WITH x AS (SELECT {expression} AS v FROM orders) SELECT v FROM x", True
        )
        yield Basic(
            "cte",
            f"WITH x AS (SELECT order_id, {expression} AS v FROM orders), "
            "y AS (SELECT * FROM x WHERE order_id > 1) SELECT v FROM y",
            True,
        )
        yield Basic("subquery", f"SELECT v FROM (SELECT {expression} AS v FROM orders) AS s", True)
    for join in JOINS:
        yield Basic(
            "join",
            f"SELECT o.order_id, c.email FROM orders o {join} customers c "
            "ON o.customer_id = c.customer_id",
            True,
        )
    for left, right in UNION_PAIRS:
        source = " FROM orders" if left[0].isalpha() else ""
        for union in UNIONS:
            yield Basic(
                "union", f"SELECT {left} AS v{source} {union} SELECT {right} AS v{source}", True
            )
    for window in WINDOWS:
        yield Basic("window", f"SELECT order_id, {window} AS w FROM orders", True)
    for shape, sql in FIXED:
        yield Basic(shape, sql, True)


def _expressions() -> Iterator[tuple[str, str]]:
    """Select-list expressions by shape: literals, columns, arithmetic, casts and functions."""
    for literal in LITERALS:
        yield "literal", literal
    for column in COLUMNS:
        yield "column", column
    for left in OPERANDS_LEFT:
        for operator in OPERATORS:
            for right in OPERANDS_RIGHT:
                yield "arithmetic", f"{left} {operator} {right}"
    yield "concat", "'a' || 'b'"
    yield "concat", "status || '!'"
    for cast in CASTS:
        for subject in CAST_SUBJECTS:
            if not cast.startswith(TRY_CAST) or subject == TRY_CAST_ONLY_SUBJECT:
                yield "cast", cast.format(x=subject)
    for function in FUNCTIONS:
        yield "function", function
