"""Recorder identity, file formats and defaults."""

from __future__ import annotations

import re

RECORDER_NAME: str = "sqlquack-record"
RECORDER_VERSION: str = "0.1.0"
EVIDENCE_FORMAT: int = 1
DEFAULT_PENDING_DIR: str = "evidence/pending"
DEFAULT_RESULTS_DIR: str = "evidence/results"
REPOSITORY_MARKER: str = "sqlquack.toml"
DEFAULT_BATCH_SIZE: int = 20
QUERY_TAG: str = "sqlquack-record"
CONNECTION_ENVIRONMENT: str = "SQLQUACK_SNOWFLAKE_CONNECTION"
WAREHOUSE_ENVIRONMENT: str = "SQLQUACK_SNOWFLAKE_WAREHOUSE"
CONNECTION_OVERRIDES: dict[str, str] = {
    "SNOWFLAKE_ACCOUNT": "account",
    "SNOWFLAKE_USER": "user",
    "SNOWFLAKE_ROLE": "role",
    "SNOWFLAKE_AUTHENTICATOR": "authenticator",
    "SNOWFLAKE_HOST": "host",
    "SNOWFLAKE_DATABASE": "database",
    "SNOWFLAKE_SCHEMA": "schema",
}
RESULT_FORMAT_PARAMETER: str = "PYTHON_CONNECTOR_QUERY_RESULT_FORMAT"
ARROW_RESULT_FORMAT: str = "ARROW"
RECORDABLE_SQLSTATE_CLASSES: frozenset[str] = frozenset({"0A", "21", "22", "42"})
DATA_EXCEPTION_SQLSTATE_CLASSES: frozenset[str] = frozenset({"22"})
SESSION_DEPENDENT_SQLSTATES: frozenset[str] = frozenset({"42501"})
SESSION_FAILURE_SQLSTATE_CLASSES: frozenset[str] = frozenset({"08", "28", "53", "57", "XX"})
HASH_NAME_LENGTH: int = 64
HEX_DIGITS: frozenset[str] = frozenset("0123456789abcdef")
PENDING_KEYS: frozenset[str] = frozenset(
    {
        "format",
        "case_hash",
        "case_reference",
        "source_dialect",
        "session_parameters",
        "source_sql",
        "duckdb_version",
    }
)
VALUES_KEY: str = "values"
SETUP_KEY: str = "setup_sql"
PINNED_KEY: str = "pinned_parameters"
OPTIONAL_PENDING_KEYS: frozenset[str] = frozenset({VALUES_KEY, SETUP_KEY, PINNED_KEY})
SHOW_SESSION_PARAMETERS: str = "SHOW PARAMETERS IN SESSION"
PARAMETER_KEY_COLUMN: str = "key"
PARAMETER_VALUE_COLUMN: str = "value"
SESSION_DEFAULTS_FORMAT: int = 1
ARGUMENT_SEPARATOR: str = ","
PARENTHESIS_DEPTH: dict[str, int] = {"(": 1, ")": -1}
SCRATCH_FUNCTION_SUFFIX: str = "__sqlquack_"
SCRATCH_HASH_LENGTH: int = 16
SCRATCH_NONCE_BYTES: int = 4
CREATE_FUNCTION_PATTERN: str = (
    r"CREATE FUNCTION (?P<name>[a-z_][a-z0-9_]*)\((?P<arguments>.*?)\)\s*RETURNS"
)
ROWS_OUTCOME: str = "rows"
ERROR_OUTCOME: str = "error"
TYPES_OUTCOME: str = "types"
ACCOUNT_ARGUMENT: str = "account"
WAREHOUSE_ARGUMENT: str = "warehouse"
SESSION_PARAMETERS_ARGUMENT: str = "session_parameters"
CREDENTIAL_CACHE_DEFAULTS: dict[str, bool] = {
    "client_store_temporary_credential": True,
    "oauth_enable_refresh_tokens": True,
}
MULTI_STATEMENT_FAILURE: tuple[int, str] = (100132, "P0000")
QUERY_ID_PATTERN: re.Pattern[str] = re.compile(
    r"\s*\(?(?:query id:\s*)?[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}\)?:?", re.IGNORECASE
)
ERROR_PREFIX_PATTERN: re.Pattern[str] = re.compile(r"^\d+ \([0-9A-Z]{5}\):\s*")
