"""Probe wrappers, file layout, raise-message patterns and limits."""

from __future__ import annotations

import re

PROBE_FUNCTION: str = "GUARD_PROBE"
SAMPLE_RELATION: str = "GUARD_SAMPLE"
UNUSED_ARGUMENT: tuple[str, str] = ("UNUSED", "NUMBER(1,0)")
RESULT_COLUMN: str = "R"
SEMI_STRUCTURED_BODY: str = "SELECT TO_JSON({expression}) IS NULL AS r FROM guard_sample"
PLAIN_BODY: str = "SELECT ({expression}) IS NULL AS r FROM guard_sample"
PARQUET_SUFFIX: str = ".parquet"
TYPES_SUFFIX: str = ".types.json"
UNKNOWN_TYPE: str = "UNKNOWN"
SEMI_STRUCTURED_TYPES: tuple[str, ...] = ("VARIANT", "OBJECT", "ARRAY")
BARE_ARGUMENT: re.Pattern[str] = re.compile(r"\s*x1\s*")
SQL_LANGUAGE: str = "sql"
BOOLEAN_TYPE: str = "BOOLEAN"
REFUSED_FALLBACK: str = "refused"

GUARD_RAISE: re.Pattern[str] = re.compile(r"'([a-z_]+): ([^']+)'")
SNOWFLAKE_RAISE: re.Pattern[str] = re.compile(r"'sqlquack:([0-9]+): ([^']+)'")
QUOTED_VALUE: re.Pattern[str] = re.compile(r"'[^']*'|\"[^\"]*\"")
EXPONENT_MARKER: re.Pattern[str] = re.compile(r"[eE]")
DECIMAL_POINT: str = "."
ZERO_DIGITS: frozenset[str] = frozenset("0")
SIX_ZEROS: str = "000000"
NUMBER_FIDELITY_REASON: str = "JSON with a number, word or comma DuckDB may read differently"
JSON_STRING: re.Pattern[str] = re.compile(r'"(?:[^"\\]|\\.)*"')
JSON_NUMBER: re.Pattern[str] = re.compile(r"-?[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?")
JSON_WORD: re.Pattern[str] = re.compile(r"\b(?!true\b|false\b|null\b)[A-Za-z]+\b")
TRAILING_COMMA: re.Pattern[str] = re.compile(r",\s*[\]}]")
DOUBLE_DIGITS: int = 15
LONG_INTEGER_DIGITS: int = 39

BODY_START: str = "CROSS JOIN LATERAL ("
BODY_END: str = ') AS "__sqlquack_body"'
SELECT_PREFIX: str = "SELECT "
RESULT_PROJECTION: str = ' AS "R" FROM '
COMMON_SUBEXPRESSIONS: str = "common_subexpressions"
RAISE_HOOK: str = "__sqlquack_raise_hook"
RAISE_SITE: re.Pattern[str] = re.compile(r"CAST\(CAST\('((?:[^']|'')*)' AS INTEGER\) AS \"NULL\"\)")

LEAF_SIZE: int = 8
DEFAULT_QUERY_LIMIT: int = 2000
DEFAULT_EXAMPLE_LIMIT: int = 100
EXAMPLE_SPREAD: int = 2654435761
EXAMPLE_MODULUS: int = 4294967296
SHAPES_PER_REASON: int = 3
SHAPE_LENGTH: int = 60
SUMMARY_COLUMNS: int = 15
SUMMARY_REASONS: int = 5
