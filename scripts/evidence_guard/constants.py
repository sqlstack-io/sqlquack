"""Evidence guard settings."""

from __future__ import annotations

RESULTS_DIR: str = "evidence/results"
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
HASH_LENGTH: int = 64
ERROR_KEYS: frozenset[str] = frozenset({"errno", "sqlstate"})
ADDED_STATUS: str = "A"
ARROW_SUFFIX: str = ".arrow"
METADATA_SUFFIX: str = ".json"
ROWS_OUTCOME: str = "rows"
ERROR_OUTCOME: str = "error"
HEX_DIGITS: frozenset[str] = frozenset("0123456789abcdef")
GIT_FIELD_SEPARATOR: str = "\t"
