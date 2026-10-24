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
OAUTH_AUTHORIZATION_CODE: str = "oauth_authorization_code"
RECORDABLE_SQLSTATE_CLASSES: frozenset[str] = frozenset({"0A", "21", "22", "42"})
HASH_NAME_LENGTH: int = 64
