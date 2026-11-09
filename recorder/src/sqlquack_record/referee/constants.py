"""Referee case formats, where they live and what a referee session may run."""

from __future__ import annotations

import re

REFEREE_FORMAT: int = 1
REFEREE_DIRECTORY: str = "referee"
CASES_DIRECTORY: str = "cases"
ENVIRONMENTS_DIRECTORY: str = "environments"
RESULTS_DIRECTORY: str = "results"
FUNCTIONS_DIRECTORY: str = "functions"
CATALOGUE_REQUEST: str = "catalogue.json"
CASE_KEYS: frozenset[str] = frozenset(
    {
        "format",
        "case_hash",
        "case_reference",
        "source_dialect",
        "environment_sha256",
        "statement",
        "syntax_only",
    }
)
ENVIRONMENT_KEYS: frozenset[str] = frozenset(
    {"format", "environment_sha256", "session_parameters", "setup_sql"}
)
ACCEPTED_OUTCOME: str = "accepted"
REJECTED_OUTCOME: str = "rejected"
SETUP_PATTERN: re.Pattern[str] = re.compile(
    r'CREATE TEMPORARY TABLE (?:[A-Za-z_][A-Za-z0-9_$]*|"[^"]+") \(', re.ASCII
)
LEADING_COMMENTS: re.Pattern[str] = re.compile(r"\A(?:\s+|--[^\n]*(?:\n|\Z)|/\*.*?\*/)*", re.DOTALL)
QUERY_START: re.Pattern[str] = re.compile(r"(?:\(|(?:SELECT|WITH|VALUES)\b)", re.IGNORECASE)
STATEMENT_SEPARATOR: str = ";"
FORBIDDEN_TEXT: tuple[str, ...] = (STATEMENT_SEPARATOR, "SYSTEM$")
SETUP_FORBIDDEN_TEXT: tuple[str, ...] = ("'", "$$", "SYSTEM$")
QUOTED_NAME_DELIMITER: str = '"'
LIST_OPEN: str = "("
LIST_CLOSE: str = ")"
SETUP_QUERY_WORD: re.Pattern[str] = re.compile(r"\bSELECT\b", re.IGNORECASE)
SYNTAX_ERROR_TEXT: str = "syntax error"
SHOW_FUNCTIONS: str = "SHOW FUNCTIONS"
BUILTIN_COLUMN: str = "is_builtin"
BUILTIN_VALUE: str = "Y"
CATALOGUE_COLUMNS: tuple[str, ...] = (
    "name",
    "arguments",
    "min_num_arguments",
    "max_num_arguments",
    "is_aggregate",
    "is_ansi",
    "is_table_function",
    "description",
)
IDENTITY_FUNCTIONS: tuple[tuple[str, str], ...] = (
    ("database", "CURRENT_DATABASE()"),
    ("schema", "CURRENT_SCHEMA()"),
    ("warehouse", "CURRENT_WAREHOUSE()"),
    ("role", "CURRENT_ROLE()"),
    ("user", "CURRENT_USER()"),
    ("account", "CURRENT_ACCOUNT()"),
)
PROGRESS_EVERY: int = 250
