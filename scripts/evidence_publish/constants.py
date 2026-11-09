"""Where SQLQuack's recorded evidence is today, and where the evidence repository keeps it."""

from __future__ import annotations

import re

PUBLIC_RECEIPT_FORMAT: str = "sqlquack-syntax-receipt/3"
PRIVATE_RECEIPT_FIELDS: tuple[str, ...] = ("run_id", "dcm_project")
QUERY_ID_FIELD: str = "query_id"
QUERY_ID_HASH_FIELD: str = "query_id_sha256"
CAPTURE_PARTS: tuple[str, str] = ("run_audit_sha256", "packet_sha256")
CAPTURE_FIELD: str = "capture_sha256"
SUPERSEDES_FIELD: str = "supersedes_receipt_sha256"
RUN_PROJECT: re.Pattern[str] = re.compile(r"\bSQLQUACK_DCMD_[0-9A-F]+(?:\.[A-Z0-9_]+){0,2}")
RUN_PROJECT_PLACEHOLDER: str = "<project>"
CAPTURE_RENAMES: dict[str, str] = {
    "dcm_definition_file": "definition_plan",
    "execute_ddl_in_run_database": "execute_in_scratch_database",
    "execute_canonical_recorder": "execute_with_recorder",
}
CASE_ID_PREFIX_RENAMES: dict[str, str] = {"d" + "73-": "missing-object-"}
MEASURE_RENAMES: dict[str, str] = {
    "dcm_precheck": "definition_plan_precheck",
    "d" + "73_missing_object_with_setup": "missing_object_with_setup",
}
BASIS_REWRITES: dict[str, str] = {
    "(B's format: measure dcm_precheck,": "(measure definition_plan_precheck,"
}
HOME_PATH: re.Pattern[str] = re.compile(r"/(?:home|Users)/[^/\s\"']+/")
HOME_PLACEHOLDER: str = "~/"
DECISION_PREFIX: re.Pattern[str] = re.compile(r"^D[0-9]{2,4}:? ")
DECISION_SUFFIX: re.Pattern[str] = re.compile(r" \(D[0-9]{2,4}\)")

REPORT_SUFFIX: str = ".report.json"
WITHDRAWN_FILE: str = "withdrawn.jsonl"
INPUTS_FILE: str = "inputs.jsonl"
UNCATALOGUED_FILE: str = "other.jsonl"
RECEIPTS_DIRECTORIES: tuple[str, ...] = ("receipts", "repeats")
HASH_MAP_FILE: str = "receipt-hashes.json"
GITIGNORE_FILE: str = ".gitignore"
GITIGNORE_TEXT: str = (
    "AGENTS.md\nCLAUDE.md\n.agents/\n.amp/\n.claude/\n.codex/\n.cursor/\n.opencode/\n"
    "worklists/\n*worklist*.md\n[l]ane-reviews/\nruns.toml\nrun-notes/\n*run-note*.md\n"
    "run-logs/\n*run-log*.md\n*hand-off*.md\n*handoff*.md\n"
)

SNOWFLAKE: str = "snowflake"
SYNTAX_ROOT: str = "coverage/syntax"
DIALECT_SYNTAX_ROOT: str = "coverage/syntax-{dialect}"
SYNTAX_DIALECTS: tuple[str, ...] = (
    SNOWFLAKE,
    "duckdb",
    "postgres",
    "sqlite",
    "bigquery",
    "databricks",
)
SYNTAX_COPIED: tuple[tuple[str, str], ...] = (("cases", "cases"), ("mapped-cases", "mapped-cases"))
COPIED: tuple[tuple[str, str], ...] = (
    ("cases", "convert/snowflake/cases"),
    ("evidence/results", "convert/snowflake/results"),
    ("evidence/audit/results", "convert/snowflake/repeats"),
    ("evidence/session_defaults.json", "convert/snowflake/session_defaults.json"),
    ("referee/generated", "referee/snowflake/cases/generated"),
    ("referee/corpora", "referee/snowflake/cases/corpora"),
    ("evidence/referee/results", "referee/snowflake/results"),
    ("evidence/referee/functions", "referee/snowflake/functions"),
)
CATALOGUE_DIRECTORY: str = "catalogue"
RECEIPTS_DIRECTORY: str = "receipts"
SYNTAX_FOLDER: str = "syntax"
MESSAGE_FIELDS: tuple[str, ...] = ("message", "inner_message")
CASE_ID_MIN_PARTS: int = 3
REPORT_WITHDRAWN: str = "withdrawn"
REPORT_INPUTS: str = "inputs"
SQL_KEY: str = "sql"
SQL_HASH_KEY: str = "sql_sha256"
INPUT_KEYS: tuple[str, ...] = ("setup", "setup_sha256", "sql", "sql_sha256")
RECEIPT_HASH_KEY: str = "receipt_sha256"
