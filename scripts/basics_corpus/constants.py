"""The basics corpus: where it lives, and the vocabulary its statements are built from."""

from __future__ import annotations

from pathlib import Path

REPOSITORY: Path = Path(__file__).resolve().parents[2]
CORPUS_FILE: str = "corpora/basics/queries.toml"
SEED: int = 320
SPELLINGS: int = 4
UNRECORDED: str = "unrecorded"

LITERALS: tuple[str, ...] = (
    "1",
    "0",
    "42",
    "1.",
    ".5",
    "1.5",
    "1e3",
    "1E-2",
    "-1",
    "+1",
    "-0.25",
    "TRUE",
    "FALSE",
    "NULL",
    "'x'",
    "'it''s'",
    "''",
    "'café'",
    "DATE '2024-01-15'",
    "TIMESTAMP '2024-01-15 10:00:00'",
)
COLUMNS: tuple[str, ...] = ("order_id", "total", "status", "ordered_at", "customer_id")
OPERATORS: tuple[str, ...] = ("+", "-", "*", "/")
OPERANDS_LEFT: tuple[str, ...] = ("1", "total", "order_id")
OPERANDS_RIGHT: tuple[str, ...] = ("1", "2", "total")
CAST_SUBJECTS: tuple[str, ...] = ("total", "1", "status")
TRY_CAST_ONLY_SUBJECT: str = "status"
TRY_CAST: str = "TRY_CAST("
CASTS: tuple[str, ...] = (
    "CAST({x} AS VARCHAR)",
    "{x}::VARCHAR",
    "CAST({x} AS NUMBER(10,2))",
    "{x}::INT",
    "TRY_CAST({x} AS VARCHAR)",
    "TO_VARCHAR({x})",
)
FUNCTIONS: tuple[str, ...] = (
    "UPPER(status)",
    "LOWER(status)",
    "LENGTH(status)",
    "TRIM(status)",
    "SUBSTR(status, 1, 3)",
    "CONCAT(status, '!')",
    "ROUND(total, 1)",
    "ROUND(total)",
    "ABS(total)",
    "FLOOR(total)",
    "CEIL(total)",
    "COALESCE(status, 'none')",
    "NVL(status, 'none')",
    "NULLIF(status, 'shipped')",
    "IFF(total > 100, 'big', 'small')",
    "DATE_TRUNC('month', ordered_at)",
    "DATEADD(day, 1, ordered_at)",
    "TO_DATE(ordered_at)",
    "YEAR(ordered_at)",
    "MONTH(ordered_at)",
    "ordered_at::DATE",
    "CASE WHEN total > 100 THEN 'big' ELSE 'small' END",
    "CASE status WHEN 'shipped' THEN 1 ELSE 0 END",
)
AGGREGATES: tuple[str, ...] = (
    "COUNT(*)",
    "COUNT(order_id)",
    "COUNT(DISTINCT customer_id)",
    "SUM(total)",
    "AVG(total)",
    "MIN(total)",
    "MAX(ordered_at)",
)
PREDICATES: tuple[str, ...] = (
    "total > 100",
    "status = 'shipped'",
    "status IN ('shipped', 'pending')",
    "total BETWEEN 10 AND 100",
    "status IS NULL",
    "status IS NOT NULL",
    "status LIKE 'sh%'",
    "NOT (total > 100)",
    "total > 10 AND status = 'shipped'",
    "total < 10 OR status IS NULL",
    "order_id IN (1, 2, 3)",
    "ordered_at >= '2026-10-03'",
    "ordered_at >= DATE '2026-10-03'",
)
DERIVED: tuple[str, ...] = ("total", "total * 2", "UPPER(status)", "1")
JOINS: tuple[str, ...] = (
    "JOIN",
    "INNER JOIN",
    "LEFT JOIN",
    "LEFT OUTER JOIN",
    "RIGHT JOIN",
    "FULL OUTER JOIN",
)
UNION_PAIRS: tuple[tuple[str, str], ...] = (
    ("1", "2"),
    ("'a'", "'b'"),
    ("total", "total * 2"),
    ("order_id", "customer_id"),
)
UNIONS: tuple[str, ...] = ("UNION ALL", "UNION")
WINDOWS: tuple[str, ...] = (
    "ROW_NUMBER() OVER (ORDER BY total)",
    "ROW_NUMBER() OVER (PARTITION BY status ORDER BY total DESC)",
    "RANK() OVER (ORDER BY total)",
    "SUM(total) OVER (PARTITION BY customer_id)",
    "LAG(total) OVER (ORDER BY ordered_at)",
    "COUNT(*) OVER ()",
)
FIXED: tuple[tuple[str, str], ...] = (
    ("star", "SELECT * FROM orders"),
    ("star", "SELECT o.* FROM orders o"),
    ("star", "SELECT * FROM orders WHERE total > 50 ORDER BY order_id"),
    (
        "join",
        "SELECT o.order_id, i.quantity, p.name FROM orders o "
        "JOIN order_items i ON i.order_id = o.order_id "
        "JOIN products p ON p.product_id = i.product_id",
    ),
    ("join", "SELECT o.order_id, c.email FROM orders o JOIN customers c USING (customer_id)"),
    (
        "join",
        "SELECT o.order_id, c.email FROM orders o, customers c WHERE o.customer_id = c.customer_id",
    ),
    (
        "join",
        "SELECT c.email, COUNT(*) AS n FROM customers c "
        "LEFT JOIN orders o ON o.customer_id = c.customer_id GROUP BY c.email",
    ),
    ("distinct", "SELECT DISTINCT status FROM orders"),
    ("having", "SELECT status, COUNT(*) AS n FROM orders GROUP BY status HAVING COUNT(*) > 1"),
    (
        "qualify",
        "SELECT order_id, status FROM orders "
        "QUALIFY ROW_NUMBER() OVER (PARTITION BY status ORDER BY total DESC) = 1",
    ),
    (
        "last_days",
        "SELECT order_id FROM orders WHERE ordered_at >= DATEADD(day, -30, CURRENT_DATE)",
    ),
    (
        "last_days",
        "SELECT order_id FROM orders WHERE ordered_at >= DATEADD(day, -30, CURRENT_TIMESTAMP())",
    ),
)

KEYWORDS: tuple[str, ...] = (
    "SELECT",
    "FROM",
    "WHERE",
    "GROUP BY",
    "ORDER BY",
    "LIMIT",
    "OFFSET",
    "AS",
    "WITH",
    "JOIN",
    "ON",
    "UNION ALL",
    "UNION",
    "HAVING",
    "QUALIFY",
    "DISTINCT",
    "OVER",
    "PARTITION BY",
    "AND",
    "OR",
    "NOT",
    "IN",
    "IS",
    "NULL",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "CAST",
    "DESC",
    "NULLS LAST",
    "LEFT",
    "INNER",
)
CLAUSES: tuple[str, ...] = (
    "FROM",
    "WHERE",
    "GROUP BY",
    "ORDER BY",
    "LIMIT",
    "HAVING",
    "QUALIFY",
    "UNION ALL",
    "JOIN",
)
LOWER_CASE: str = "lower"
UPPER_CASE: str = "upper"
MIXED_CASE: str = "mixed"
LOWER_KEYWORDS: str = "keywords"
CASINGS: tuple[str, ...] = (LOWER_CASE, UPPER_CASE, MIXED_CASE, LOWER_KEYWORDS)
UNSPACED: str = "none"
LEADING: str = "lead"
TRAILING: str = "trail"
BLOCK: str = "block"
COMMENTS: tuple[str | None, ...] = (None, None, LEADING, TRAILING, BLOCK)
HALF: float = 0.5
ENDING_SHARE: float = 0.4
QUOTED_TABLE_SHARE: float = 0.25
QUOTE: str = "'"
SPACINGS: dict[str, str] = {"spaces": "   ", "tabs": "\t", "newlines": "\n  "}
ENDINGS: tuple[str, ...] = (";", " ;", ";\n", "\n")
LEADING_COMMENT: str = "-- a comment\n"
TRAILING_COMMENT: str = " -- trailing comment"
BLOCK_COMMENT: str = " /* c */ "
QUOTED_TABLE: str = '"ORDERS"'

HEADER: str = """# Generated by scripts/generate_basics_corpus.py, then `make corpus-rerecord`.
# Do not edit. Basic Snowflake statements a first-time user types, each in its plain spelling
# and others that mean the same statement (case, whitespace, a final semicolon, comments, a
# quoted table name), with the outcome SQLQuack gives each under the built-in policy (`expect`)
# and under the repository policy in a UTC session (`expect_repository`), and the translation's
# result on fixture.sql. The everyday corpus test checks them.
"""
