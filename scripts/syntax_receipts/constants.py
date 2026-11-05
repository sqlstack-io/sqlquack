"""Settings for importing Snowflake capture runs as syntax-coverage receipts."""

from __future__ import annotations

RECEIPT_FORMAT: str = "sqlquack-syntax-receipt/1"
PHASES: frozenset[str] = frozenset(
    {"accepted", "syntax_error", "semantic_error", "runtime_error", "unsupported_feature"}
)
SYNTAX_ERRNO: int = 1003
UNSUPPORTED_ERRNO: int = 2
COMPILATION_PREFIX: str = "SQL compilation error"
UNCAUGHT_PREFIX: str = "Uncaught exception of type"
UNSUPPORTED_PREFIX: str = "Unsupported feature"
RUNTIME_ERRNO_FLOOR: int = 100000
SYNTAX_OR_ACCESS_SQLSTATE_CLASS: str = "42"
REVIEWED_SEMANTIC_ERRNOS: frozenset[int] = frozenset({391858, 392002, 506005, 506034, 506035})
SCRIPTING_SUBMISSION: str = "anonymous block, as-is"
EXPLAIN_METHOD: str = "EXPLAIN USING TEXT, once"
CAPTURE_EXECUTE: str = "execute"
CAPTURE_EXPLAIN: str = "compile_only_explain"
CAPTURE_CANONICAL: str = "execute_canonical_recorder"
CAPTURE_DDL: str = "execute_ddl_in_run_database"
CONTROLLER_CATEGORIES: frozenset[str] = frozenset({"metadata"})
RUN_DATABASE: str = "<run_database>"
RUN_NONCE: str = "<run_nonce>"
CASE_SCHEMA: str = "<case_schema>"
SHARED_SCHEMA_NOTE: str = "-- shared schema; objects persist from this run's earlier cases:"
PACKET_SUFFIXES: tuple[str, ...] = (".jsonl", ".json")
START_EVENT: str = "start"
RECORDING_SESSION_EVENT: str = "recording_session"
COMPLETE_EVENT: str = "complete"
RECONCILIATION_FILE: str = "reconciliation.json"
RECONCILIATION_FORMAT: str = "sqlquack-run-reconciliation/1"
RECONCILED_STATUS: str = "reconciled"
UNRECORDED_KEY: str = "unrecorded_submission"
QUERY_HISTORY_SOURCE: str = "query_history"
QUERY_HISTORY_SUCCESS: str = "SUCCESS"
QUERY_HISTORY_FAILED: str = "FAILED_WITH_ERROR"
CLEANUP_SUCCESS: str = "SUCCESS"
SQL_KEY: str = "sql"
RUN_DATABASE_KEY: str = "run_database"
PROPOSAL_KEY: str = "proposal_sha256"
MANIFEST_KEY: str = "manifest_sha256"
PACKET_ACCEPTED_OUTCOMES: tuple[str, ...] = ("accepted_executed", "compiled")
DIAGNOSTIC_ACCEPTED_OUTCOME: str = "rows"
CANONICAL_ACCEPTED_OUTCOMES: tuple[str, ...] = ("rows", "types")
SETUP_FAILED_OUTCOME: str = "setup_failed"
WHOLE_RUN_ROW: str = "*"
UNKNOWN_ROW: str = "?"
UNCATEGORIZED_PHASES: frozenset[str] = frozenset({"accepted", "syntax_error"})
MISSING_OBJECT: str = "missing_object"
INVALID_OPTION_OR_VALUE: str = "invalid_option_or_value"
TYPE_REASON: str = "type"
OTHER_REASON: str = "other"
REASON_BY_ERRNO: dict[int, str] = {
    **dict.fromkeys(
        (
            709,
            904,
            1431,
            2003,
            2043,
            2140,
            2141,
            2142,
            2211,
            3620,
            90080,
            91083,
            91085,
            91471,
            392002,
        ),
        MISSING_OBJECT,
    ),
    **dict.fromkeys(
        (
            1004,
            1006,
            1008,
            1420,
            1422,
            1424,
            1521,
            1867,
            2006,
            2029,
            2134,
            2135,
            2308,
            3058,
            3071,
            3089,
            3143,
            3618,
            4613,
            92207,
        ),
        INVALID_OPTION_OR_VALUE,
    ),
    **dict.fromkeys((1007, 1044, 1421, 3551, 506034, 90201, 90207, 92229), TYPE_REASON),
}
MISSING_OBJECT_MESSAGE: str = r"does not exist|\bnonexistent\b|\bnot found\b"
SUPERSEDES_KEY: str = "supersedes_receipt_sha256"
MEASURE_KEY: str = "measure"
