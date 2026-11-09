"""Evidence guard settings."""

from __future__ import annotations

RESULTS_PATHSPECS: tuple[str, ...] = (
    "evidence/results",
    "evidence/audit/results",
    "evidence/referee",
    "coverage/syntax/receipts",
    ":(glob)coverage/syntax-*/receipts/**",
)
UNGUARDED_FILES: frozenset[str] = frozenset({"README.md"})
RECORDER_PREFIX: str = "sqlquack-record "
EVIDENCE_FORMAT: int = 1
METADATA_KEYS: frozenset[str] = frozenset(
    {
        "format",
        "case_hash",
        "outcome",
        "recorder",
        "connector",
        "query_id_sha256",
        "warehouse_version",
        "duckdb_version",
        "recorded_at",
        "session_parameters",
        "columns",
        "row_count",
        "arrow_sha256",
        "error",
    }
)
OPTIONAL_METADATA_KEYS: frozenset[str] = frozenset({"effective_parameters", "parameters_sha256"})
HASH_LENGTH: int = 64
ERROR_KEYS: frozenset[str] = frozenset({"errno", "sqlstate"})
ERROR_MESSAGE_KEY: str = "message"
ADDED_STATUS: str = "A"
ARROW_SUFFIX: str = ".arrow"
METADATA_SUFFIX: str = ".json"
ROWS_OUTCOME: str = "rows"
ERROR_OUTCOME: str = "error"
TYPES_OUTCOME: str = "types"
HEX_DIGITS: frozenset[str] = frozenset("0123456789abcdef")
GIT_FIELD_SEPARATOR: str = "\t"
REFEREE_RESULTS_PREFIX: str = "evidence/referee/results/"
REFEREE_FUNCTIONS_PREFIX: str = "evidence/referee/functions/"
REFEREE_RESULT_KEYS: frozenset[str] = frozenset(
    {
        "format",
        "case_hash",
        "environment_sha256",
        "syntax_only",
        "outcome",
        "recorder",
        "connector",
        "warehouse_version",
        "recorded_at",
        "effective_parameters",
        "error",
        "columns",
        "describe_error",
    }
)
REFEREE_CATALOGUE_KEYS: frozenset[str] = frozenset(
    {
        "format",
        "catalogue_sha256",
        "recorder",
        "connector",
        "warehouse_version",
        "recorded_at",
        "functions",
    }
)
ACCEPTED_OUTCOME: str = "accepted"
REJECTED_OUTCOME: str = "rejected"
REFEREE_DIRECTORY: str = "evidence/referee/"
SYNTAX_RECEIPTS_PATTERN: str = r"coverage/syntax(-[^/]+)?/receipts/"
SYNTAX_RECEIPT_SUFFIX: str = ".jsonl"
SYNTAX_REPORT_SUFFIX: str = ".report.json"
