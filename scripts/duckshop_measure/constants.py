"""Relation naming, DuckDB session settings, test-result columns and report labels."""

from __future__ import annotations

import re

TABLE_PATTERN: re.Pattern[str] = re.compile(r"CREATE TEMPORARY TABLE (\w+) \((.*)\)\s*$", re.S)
NOT_NULL: re.Pattern[str] = re.compile(r"\s+NOT NULL\s*$")
ENVIRONMENT_PREFIX: str = "rel__"
CENSUS_RELATION_PREFIX: str = "__SQLQUACK_RELATION__"
OPEN_PARENTHESIS: str = "("
CLOSE_PARENTHESIS: str = ")"
TOP_LEVEL_SEPARATOR: str = ","
IDENTIFIER_QUOTE: str = '"'
REPLAY_SESSION: str = (
    "SET TimeZone = 'UTC'; SET default_null_order = 'nulls_last_on_asc_first_on_desc'; "
    "SET threads = 1;"
)
ACTUAL_COUNT: int = 2
EXPECTED_COUNT: int = 3
UNEXPECTED_COUNT: int = 4
MISSING_COUNT: int = 5
GUARD_RAISE: re.Pattern[str] = re.compile(r"'([a-z_]+)#[0-9a-f]{6,}")
TESTS_DIR: str = "tests"
SQL_SUFFIX: str = ".sql"
PROJECT_COMMAND: str = "project"
CENSUS_SOURCES: str = "census.yml"
TESTS_COMMAND: str = "tests"
TESTS_LABEL: str = "matches SQLBuild test expectations"
SNOWFLAKE_COMMAND: str = "snowflake"
SNOWFLAKE_LABEL: str = "matches Snowflake"
CENSUS_TABLE_PREFIX: str = "__sqlquack_relation__"
COMPILED_PREFIX: str = "target/compiled/"
MISSING_TABLE: re.Pattern[str] = re.compile(
    r"(?:Table|Scalar Function|Table Function) with name "
    r"(__sqlquack_(?:relation|udf|table_fn)__\w+) does not exist",
    re.IGNORECASE,
)
MICROSECOND_SCALE: int = 6
SCALED_TYPES: frozenset[str] = frozenset({"TIME", "TIMESTAMP_NTZ", "TIMESTAMP_LTZ", "TIMESTAMP_TZ"})
SNOWFLAKE_SPELLINGS: dict[str, str] = {"REAL": "FLOAT", "TEXT": "VARCHAR"}
DUCKDB_TYPES: dict[str, str] = {
    "REAL": "DOUBLE",
    "TEXT": "VARCHAR",
    "BOOLEAN": "BOOLEAN",
    "DATE": "DATE",
    "TIME": "TIME_NS",
    "TIMESTAMP_TZ": "TIMESTAMPTZ",
    "TIMESTAMP_LTZ": "TIMESTAMPTZ",
    "VARIANT": "VARCHAR",
    "OBJECT": "VARCHAR",
    "ARRAY": "VARCHAR",
    "BINARY": "BLOB",
}
NOT_SCORED_EXECUTIONS: frozenset[str] = frozenset({"materialized_relation"})
MODEL_PREFIX: str = "model."
TARGETS: tuple[str, ...] = ("core", "edge")
SCORE_EXCLUDED: frozenset[str] = frozenset(
    {
        "nondeterministic",
        "incremental",
        "unrecordable",
        "snowflake_error",
        "input_unrecorded",
        "input_unloadable",
        "declared_divergence",
        "python_udf_not_measured",
    }
)
WORKERS: int = 3
BODY_DELIMITER: str = "$$"
PYTHON_LANGUAGE: str = "python"
UNRESOLVED_UDF: re.Pattern[str] = re.compile(r'__udf\(\s*"([^"]+)"\s*\)')
TOP_CONSTRUCTS: int = 15
JSON_SUFFIX: str = ".json"
VALUE_FAILURE: str = "value"
QUOTE_CHARACTERS: str = "'\""
WORD_JOINER: str = "_"
ORDER_BY: tuple[str, str] = ("ORDER", "BY")
FIXED_TYPE: str = "FIXED"
TIMESTAMP_NTZ_TYPE: str = "TIMESTAMP_NTZ"

EVIDENCE_FILE: str = "crates/sqlquack-policy/policy.toml"
