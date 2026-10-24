"""Recorder identity, file formats and defaults."""

from __future__ import annotations

RECORDER_NAME: str = "sqlquack-record"
RECORDER_VERSION: str = "0.1.0"
EVIDENCE_FORMAT: int = 1
DEFAULT_PENDING_DIR: str = "evidence/pending"
DEFAULT_RESULTS_DIR: str = "evidence/results"
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
}
RECORDABLE_SQLSTATE_CLASSES: frozenset[str] = frozenset({"0A", "21", "22", "42"})
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
ROWS_OUTCOME: str = "rows"
ERROR_OUTCOME: str = "error"
ACCOUNT_ARGUMENT: str = "account"
WAREHOUSE_ARGUMENT: str = "warehouse"
SESSION_PARAMETERS_ARGUMENT: str = "session_parameters"
CREDENTIAL_CACHE_DEFAULTS: dict[str, bool] = {
    "client_store_temporary_credential": True,
    "oauth_enable_refresh_tokens": True,
}
MULTI_STATEMENT_FAILURE: tuple[int, str] = (100132, "P0000")
