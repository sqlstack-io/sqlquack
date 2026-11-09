"""Queries over `orders` combining select lists with WHERE, GROUP BY, HAVING, QUALIFY and more."""

from __future__ import annotations

from itertools import product

from scripts.referee_cases._helpers.unique import unique_statements
from scripts.referee_cases.constants import SCENARIO_KIND
from scripts.referee_cases.models import Statement

_SELECTS: tuple[tuple[str, str], ...] = (
    ("plain", "order_id, status"),
    ("aggregate", "region, SUM(amount) AS total"),
    ("window", "order_id, ROW_NUMBER() OVER (PARTITION BY region ORDER BY placed_at) AS rn"),
    ("star", "*"),
    ("expression", "amount * 2 AS doubled, doubled + 1 AS bumped"),
)
_WHERES: tuple[tuple[str, str], ...] = (
    ("none", ""),
    ("column", " WHERE amount > 10"),
    ("aggregate_alias", " WHERE total > 10"),
    ("window_alias", " WHERE rn = 1"),
    ("lateral_alias", " WHERE doubled > 10"),
)
_GROUPS: tuple[tuple[str, str], ...] = (
    ("none", ""),
    ("column", " GROUP BY region"),
    ("ordinal", " GROUP BY 1"),
    ("all", " GROUP BY ALL"),
    ("two_columns", " GROUP BY region, status"),
    ("alias", " GROUP BY doubled"),
)
_HAVINGS: tuple[tuple[str, str], ...] = (
    ("none", ""),
    ("aggregate", " HAVING SUM(amount) > 100"),
    ("alias", " HAVING total > 100"),
    ("column", " HAVING region = 'north'"),
)
_QUALIFIES: tuple[tuple[str, str], ...] = (
    ("none", ""),
    ("alias", " QUALIFY rn = 1"),
    ("window", " QUALIFY ROW_NUMBER() OVER (ORDER BY order_id) = 1"),
    ("plain", " QUALIFY order_id > 1"),
)
_ORDERS: tuple[tuple[str, str], ...] = (
    ("none", ""),
    ("ordinal", " ORDER BY 1"),
    ("column_nulls", " ORDER BY order_id DESC NULLS LAST"),
    ("alias", " ORDER BY total"),
    ("unselected", " ORDER BY placed_at"),
    ("ordinal_out_of_range", " ORDER BY 9"),
)
_LIMITS: tuple[tuple[str, str], ...] = (
    ("none", ""),
    ("limit", " LIMIT 5"),
    ("offset", " LIMIT 5 OFFSET 2"),
    ("fetch", " OFFSET 2 ROWS FETCH NEXT 5 ROWS ONLY"),
)
_DISTINCTS: tuple[tuple[str, str], ...] = (("all", ""), ("distinct", "DISTINCT "))
_CLAUSE_ORDER: tuple[str, ...] = ("where", "group", "having", "qualify", "order", "limit")


def clause_statements() -> tuple[Statement, ...]:
    """Three deterministic grids of clause combinations, without repeated SQL."""
    grids: list[tuple[tuple[str, tuple[str, str]], ...]] = [
        (("select", select), ("where", where), ("group", group))
        for select, where, group in product(_SELECTS, _WHERES, _GROUPS)
    ]
    grids.extend(
        (("select", select), ("group", group), ("having", having), ("qualify", qualify))
        for select, group, having, qualify in product(_SELECTS, _GROUPS[:3], _HAVINGS, _QUALIFIES)
    )
    grids.extend(
        (("distinct", distinct), ("select", select), ("order", order), ("limit", limit))
        for distinct, select, order, limit in product(_DISTINCTS, _SELECTS, _ORDERS, _LIMITS)
    )
    return tuple(unique_statements(map(_statement, grids)))


def _statement(parts: tuple[tuple[str, tuple[str, str]], ...]) -> Statement:
    chosen: dict[str, str] = {clause: text for clause, (_, text) in parts}
    sql: str = "".join(
        (
            f"SELECT {chosen.get('distinct', '')}{chosen['select']} FROM orders",
            *(chosen.get(clause, "") for clause in _CLAUSE_ORDER),
        )
    )
    name: str = "/".join(f"{clause}_{label}" for clause, (label, _) in parts)
    return Statement(name=name, sql=sql, kind=SCENARIO_KIND)
