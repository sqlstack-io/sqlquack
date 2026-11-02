"""Where referee cases, pending files and recorded outcomes live, and their formats."""

from __future__ import annotations

import re

REFEREE_DIRECTORY: str = "referee"
ENVIRONMENTS_DIRECTORY: str = "environments"
PENDING_DIRECTORY: str = "evidence/pending/referee"
EVIDENCE_DIRECTORY: str = "evidence/referee"
RESULTS_DIRECTORY: str = "results"
FUNCTIONS_DIRECTORY: str = "functions"
CASES_DIRECTORY: str = "cases"
CATALOGUE_REQUEST: str = "catalogue.json"
REFEREE_FORMAT: int = 1
SOURCE_DIALECT: str = "snowflake"
JSON_SUFFIX: str = ".json"
HASH_LENGTH: int = 64
HEX_DIGITS: frozenset[str] = frozenset("0123456789abcdef")
SCENARIO_KIND: str = "scenario"
RELATION_KINDS: frozenset[str] = frozenset({"model", "source", "seed"})
COMMENT_LINE: re.Pattern[str] = re.compile(r"^\s*--")
RELATION_PLACEHOLDER: re.Pattern[str] = re.compile(r'__(?:ref|source|seed)\("([^"]+)"\)')
PLACEHOLDER: re.Pattern[str] = re.compile(r"\b__[a-z_]+\(")
CURSOR_START: tuple[str, str] = ("__cursor_start()", "TO_TIMESTAMP_NTZ('2026-01-01 00:00:00')")
CURSOR_END: tuple[str, str] = ("__cursor_end()", "TO_TIMESTAMP_NTZ('2026-01-02 00:00:00')")
SQLGLOT_CALLS: frozenset[str] = frozenset({"validate_identity", "validate_all"})
QUERY_START: re.Pattern[str] = re.compile(r"\s*(?:\(|(?:SELECT|WITH|VALUES)\b)", re.IGNORECASE)
SYNTAX_ONLY_EXCLUSIONS: re.Pattern[str] = re.compile(
    r";|SYSTEM\$|@|\$[A-Za-z_]|INFORMATION_SCHEMA|ACCOUNT_USAGE|RESULT_SCAN|"
    r"\b(?:FROM|JOIN|TABLE|INTO)\s+[A-Za-z_\"][\w\"$]*\.",
    re.IGNORECASE,
)
CORPORA_DIRECTORY: str = "corpora"
SQLGLOT_CORPUS: str = "sqlglot"
SQLGLOT_ORIGIN: str = "sqlglot tests/dialects/test_snowflake.py"
SESSION_PARAMETERS: tuple[tuple[str, bool | int | str], ...] = (
    ("QUOTED_IDENTIFIERS_IGNORE_CASE", False),
    ("TIMESTAMP_TYPE_MAPPING", "TIMESTAMP_NTZ"),
    ("TIMEZONE", "UTC"),
)
MODEL_KIND: str = "model"
STATEMENT_SEPARATOR: str = ";"
PARSE_FIDELITY: str = "parse fidelity"
DOES_NOT_PARSE: str = "the SQL does not parse"
SYNTAX_ERRNO: int = 1003
SYNTAX_ERROR_TEXT: str = "syntax error"
ACCEPTED_OUTCOME: str = "accepted"
CONVERT_ARGUMENTS: tuple[str, ...] = ("convert", "--builtin-policy", "--json", "-")
CHECK_WORKERS: int = 2
BOTH_ACCEPT: str = "accepted by both"
TOO_STRICT: str = "SQLQuack too strict (parse rejects what Snowflake accepts)"
TOO_LENIENT: str = "SQLQuack too lenient on syntax"
BOTH_SYNTAX: str = "syntax error caught"
CAUGHT_AT_PARSE: str = "binding error caught, at parse (kind differs)"
MISSED: str = "missed error (Snowflake binding error, SQLQuack parses)"
NOT_RECORDED: str = "not recorded"
ARITY_FUNCTION: re.Pattern[str] = re.compile(
    r"((?:\"[a-z0-9_]+\"\s*\|?\s*)+)=>\s*\((\d+),\s*(Some\((\d+)\)|None)\)"
)
ARITY_NAME: re.Pattern[str] = re.compile(r"\"([a-z0-9_]+)\"")
SIGNATURES_SOURCE: str = "crates/sqlquack-semantics/src/lineage/_helpers/checks/signatures.rs"
REASON_NOISE: re.Pattern[str] = re.compile(r"'[^']*'|\"[^\"]*\"|`[^`]*`|\[[^\]]*\]|\d+")
EXAMPLES_PER_PATTERN: int = 1
TOP_PATTERNS: int = 15
VALID_KIND: str = "valid"
SWEEP_KINDS: frozenset[str] = frozenset({"arity", "types", "valid"})
FUNCTIONS_FILE: str = "generated/functions.json#"
VARIADIC_MARKER: str = "..."
RETURN_MARKER: str = " RETURN "
RELATION_PREFIX: str = "rel__"
SYSTEM_FUNCTION: str = "SYSTEM$"
