"""Header patterns, the census's placeholder names, and the project folder files they become."""

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
PYTHON_LANGUAGE: str = "python"
PYTHON_SUFFIX: str = ".py"
FUNCTIONS_FOLDER: str = "functions"
SOURCES_FOLDER: str = "sources"
LANGUAGE_FOLDERS: dict[str, str] = {SQL_LANGUAGE: "sql", PYTHON_LANGUAGE: "python"}
LANGUAGE_SUFFIXES: dict[str, str] = {SQL_LANGUAGE: FUNCTION_SUFFIX, PYTHON_LANGUAGE: PYTHON_SUFFIX}
PLAIN_IDENTIFIER: re.Pattern[str] = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
PLACEHOLDER_BODY: str = "NULL"
IDENTIFIER_QUOTE: str = '"'
