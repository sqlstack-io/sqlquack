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
        "query_id",
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
