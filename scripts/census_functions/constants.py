"""Header patterns and the placeholder names the census gives project function references."""

from __future__ import annotations

import re

HEADER_PATTERN: re.Pattern[str] = re.compile(
    r"CREATE\s+(?:OR\s+REPLACE\s+)?(?:TEMP(?:ORARY)?\s+)?FUNCTION\s+[^(]+\((?P<arguments>.*?)\)\s*"
    r"RETURNS\s+(?P<returns>.*?)\bAS\s+\$\$",
    re.IGNORECASE | re.DOTALL,
)
TABLE_PREFIX: re.Pattern[str] = re.compile(r"TABLE\s*\(", re.IGNORECASE)
SCALAR_TYPE: re.Pattern[str] = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:\([0-9, ]+\))?")
OPEN_PAREN: str = "("
CLOSE_PAREN: str = ")"
SEPARATOR: str = ","
LANGUAGE_PATTERN: re.Pattern[str] = re.compile(r"\bLANGUAGE\s+([A-Za-z]+)", re.IGNORECASE)
SQL_LANGUAGE: str = "sql"
SCALAR_PLACEHOLDER: str = "__SQLQUACK_UDF__"
TABLE_PLACEHOLDER: str = "__SQLQUACK_TABLE_FN__"
FUNCTION_SUFFIX: str = ".sql"
RELATIONS_KEY: str = "relations"
FUNCTIONS_KEY: str = "functions"
