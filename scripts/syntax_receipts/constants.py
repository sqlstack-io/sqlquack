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
SQL_KEY: str = "sql"
RUN_DATABASE_KEY: str = "run_database"
PROPOSAL_KEY: str = "proposal_sha256"
MANIFEST_KEY: str = "manifest_sha256"
PACKET_ACCEPTED_OUTCOMES: tuple[str, ...] = ("accepted_executed", "compiled")
DIAGNOSTIC_ACCEPTED_OUTCOME: str = "rows"
CANONICAL_ACCEPTED_OUTCOMES: tuple[str, ...] = ("rows", "types")
DDL_ACCEPTED_OUTCOME: str = "accepted_executed"
WHOLE_RUN_ROW: str = "*"
UNKNOWN_ROW: str = "?"
