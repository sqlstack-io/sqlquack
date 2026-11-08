"""The BigQuery referee's fixed names: the project, capture, phases, error classes and policy."""

from __future__ import annotations

import hashlib
from pathlib import Path

from scripts.bigquery_receipts.models import ErrorClass

ENGINE: str = "bigquery"
ENGINE_VERSION: str = "unversioned service (BigQuery API v2, GoogleSQL)"
ENGINE_IDENTITY: str = "bigquery v2 jobs.insert, location US, useLegacySql false"
ENGINE_IMAGE_SHA256: str = hashlib.sha256(ENGINE_IDENTITY.encode()).hexdigest()
RECEIPT_FORMAT: str = "sqlquack-syntax-receipt/2"
RUN_AUDIT_FORMAT: str = "sqlquack-bigquery-run-audit/1"
CAPTURE: str = "execute_autocommit_in_throwaway_database"
PARSER_CHECK: str = "whole_text"
RUN_PREFIX: str = "bigquery-"

PROJECT_VARIABLE: str = "GCP_BQ_SANDBOX_PROJECT"
PROJECT_ID_PATTERN: str = r"^[a-z][a-z0-9-]{4,28}[a-z0-9]$"
LOCATION: str = "US"
KEY_VARIABLE: str = "GCP_BQ_SANDBOX_SA_KEY"
JSON_OPEN: str = "{"
SCOPE: str = "https://www.googleapis.com/auth/bigquery"
API_ROOT: str = "https://bigquery.googleapis.com/bigquery/v2"
HTTP_TIMEOUT_SECONDS: float = 60.0
WORKERS: int = 8

MAXIMUM_BYTES_BILLED: str = "10485760"
JOB_TIMEOUT_MS: str = "60000"
POLL_SECONDS: float = 1.0
POLL_LIMIT_SECONDS: float = 120.0
TABLE_EXPIRATION_MS: str = "3600000"

RUN_DATASET_PREFIX: str = "sqlquack_run_"
SECOND_DATASET_SUFFIX: str = "_2"
RUN_DATASET_PATTERN: str = r"^sqlquack_run_[a-z0-9_]+$"
NEVER_TOUCHED_PREFIXES: tuple[str, ...] = ("sqlload_",)
RUN_LABEL: str = "sqlquack_run"
JOB_PREFIX: str = "sqlquack_"

TOKEN_PROJECT: str = "SQLQUACK_PROJECT"
TOKEN_DATASET: str = "SQLQUACK_DATASET"
TOKEN_DATASET2: str = "SQLQUACK_DATASET2"
TOKEN_DASHED_PROJECT: str = "sqlquack-project"
TOKEN_PATTERN: str = (
    r"(?<![A-Za-z0-9_])SQLQUACK_(?:PROJECT|DATASET2|DATASET)(?![A-Za-z0-9_])"
    r"|(?<![A-Za-z0-9_-])sqlquack-project(?![A-Za-z0-9_-])"
)
URL_PATTERN: str = r"[A-Za-z][A-Za-z0-9+.-]*://\S+"
EMAIL_PATTERN: str = r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+"
JOB_ID_PATTERN: str = r"\b(?:sqlquack_[0-9a-f]{32}|job_[A-Za-z0-9_-]{8,}|script_job_[A-Za-z0-9_-]+)"
POSITION_PATTERN: str = r"\[(\d+):(\d+)\]"
ADDRESS_PATTERN: str = r"[A-Za-z0-9_]@[A-Za-z0-9_]"
URL_MARK: str = "<url>"
PROJECT_NUMBER_PATTERN: str = r"(?<![A-Za-z0-9_.-])\d{6,}(?=:)"
SNAPSHOT_PATTERN: str = r"@\d{10,}\b"
SNAPSHOT_MARK: str = "@<snapshot>"
READ_BEFORE_PATTERN: str = r"(Cannot read before )\d{10,}\b"
READ_BEFORE_MARK: str = r"\g<1><time>"
EMAIL_MARK: str = "<email>"
JOB_MARK: str = "<job>"

ACCEPTED: str = "accepted"
SYNTAX_ERROR: str = "syntax_error"
SEMANTIC_ERROR: str = "semantic_error"
RUNTIME_ERROR: str = "runtime_error"
OUTCOME_ACCEPTED: str = "accepted"
OUTCOME_ERROR: str = "error"
BASIS_DRY: str = "dry run raised {reason} ({code})"
BASIS_DRY_ANY: str = "dry run raised {reason} ({code}) after {first}"
BASIS_DRY_CLEAN: str = "dry run accepted the text; a query the dry run settles, not executed"
BASIS_EXECUTE: str = "execution raised {reason} ({code})"
BASIS_ROLLED_BACK: str = "execution inside a rolled-back transaction raised {reason} ({code})"
BASIS_RAN: str = "dry run accepted the text and it ran without error"
BASIS_RAN_ROLLED_BACK: str = "dry run accepted the text and it ran in a rolled-back transaction"

MODE_DRY_ONLY: str = "dry_run_only"
MODE_EXECUTE: str = "execute"
MODE_ROLLED_BACK: str = "execute_rolled_back"
MODE_HELD: str = "held"
MODE_CASE_DATASETS: str = "execute_in_case_datasets"
MODE_TIMED: str = "execute_until_stopped"
BASIS_CASE_DATASETS: str = "execution in the case's own datasets raised {reason} ({code})"
BASIS_RAN_CASE_DATASETS: str = "dry run accepted the text and it ran in the case's own datasets"
BASIS_TIMED: str = "execution ran until it was stopped: {reason} ({code})"
TIMED_PATTERN: str = r"\bLOOP\b|\bWHILE\s+TRUE\b"
TIMED_TIMEOUT_MS: str = "10000"
TIMED_CANCEL_SECONDS: float = 30.0
CASE_DATASET_FAMILIES: frozenset[str] = frozenset({"ddl", "dml"})
CASE_DATASET_PATTERN: str = r"\bROW\s+ACCESS\s+POLIC(?:Y|IES)\b"
CASE_DATASET_GUARD: str = (
    r"\bregion-|\bSCHEMA\s+(?:IF\s+(?:NOT\s+)?EXISTS\s+)?+"
    r"(?!(?:`SQLQUACK_PROJECT`\s*\.\s*)?SQLQUACK_DATASET2?(?![A-Za-z0-9_]))"
)
_OWN_DATASET: str = (
    r"(?!(?:`SQLQUACK_PROJECT`\s*\.\s*|`SQLQUACK_PROJECT\.)?`?SQLQUACK_DATASET2?(?![A-Za-z0-9_]))"
)
_NAME_PART: str = r"(?:`[^`]*`|[A-Za-z_][A-Za-z0-9_-]*)"
CASE_DATASET_TARGET: str = (
    r"\b(?:TABLE|VIEW|SCHEMA|FUNCTION|PROCEDURE|GRAPH|MODEL|INTO|UPDATE|FROM|MERGE|TO|CLONE|COPY|"
    r"LIKE|OF|INSERT|DELETE)\s+(?:IF\s+(?:NOT\s+)?EXISTS\s+)?+"
    + _OWN_DATASET
    + r"(?:`[^`]*\.[^`]*`|"
    + _NAME_PART
    + r"\s*\.)"
    + r"|\bON\s+"
    + _OWN_DATASET
    + r"(?:`[^`]*\.[^`]*`|"
    + _NAME_PART
    + r"(?:\s*\.\s*"
    + _NAME_PART
    + r")+)"
    + r"\s*(?:\(|\bFILTER\b|\bOPTIONS\b|;|$)"
)
CASE_DATASET_HELD: str = (
    "the statement names a dataset or regional resource outside the case's own datasets"
)
EXPORT_PATTERN: str = r"\bEXPORT\s+DATA\b"
EXPORT_URI: str = r"[A-Za-z][A-Za-z0-9+.-]*://[^'\"\s]*"
EXPORT_BUCKET: str = "gs://b/"
EXPORT_HELD: str = "EXPORT DATA writes to a bucket other than the sandbox's placeholder gs://b/"
CASE_SUFFIX: str = "_c{index:03d}"
SPEND_STOP_BYTES: int = 1073741824
CASE_BYTES_KEY: str = "case_bytes_billed"
ROLLED_BACK_PREFIX: str = "BEGIN TRANSACTION;\n"
ROLLED_BACK_SUFFIX: str = ";\nROLLBACK TRANSACTION;"

EXECUTION_FAMILIES: frozenset[str] = frozenset(
    {"utility", "transaction_session", "dml", "ddl", "security_admin"}
)
EXECUTED_FAMILIES: frozenset[str] = frozenset({"utility", "transaction_session"})
ROLLED_BACK_PATTERN: str = r"^\s*(?:INSERT|UPDATE|DELETE|MERGE)\b"
HELD: tuple[tuple[str, str], ...] = (
    (r"\bLOAD\s+DATA\b", "LOAD DATA reads files from a Cloud Storage bucket"),
    (r"\bEXPORT\s+MODEL\b", "EXPORT MODEL writes a trained model to a Cloud Storage bucket"),
    (
        r"\bCREATE\b[\s\S]*\bMODEL\b",
        "CREATE MODEL trains a model (paid BigQuery ML training), or needs a remote endpoint, a "
        "connection or model files in a bucket",
    ),
    (r"\bREMOTE\b", "a remote function or model needs a connection and an endpoint"),
    (r"\bCONNECTION\b", "the statement needs a connection resource"),
    (r"(?<!DROP )\bEXTERNAL\b", "an external table or schema needs a bucket or a connection"),
    (r"\bBQ\s*\.\s*JOBS\b", "BQ.JOBS procedures act on the project's jobs, outside the run"),
    (r"\bBQ\s*\.\s*ABORT_SESSION\b", "BQ.ABORT_SESSION acts on sessions outside the run"),
    (
        r"\b(?:RESERVATION|CAPACITY|ASSIGNMENT|BI_CAPACITY)\b",
        "reservations, capacity commitments and BI Engine are billed features",
    ),
)
ROLLED_BACK_FAMILY: str = "dml"
FAMILY_HELD: dict[str, str] = {
    "security_admin": (
        "the statement acts on the project, IAM, reservations or data policies, outside the "
        "run datasets"
    ),
}
EXECUTION_GUARD: str = (
    r"\b(?:CREATE|ALTER|DROP|UNDROP|TRUNCATE|GRANT|REVOKE|EXPORT|LOAD|INFORMATION_SCHEMA|"
    r"EXTERNAL_QUERY)\b|region-"
)
OUTSIDE_HELD: str = "the dry run reports a table or routine outside the run datasets"
GUARD_HELD: str = "the text names a statement or object the execution guard refuses"
INTERNAL_HELD: str = "every retry of the dry run gave HTTP 500 internalError"
INTERNAL_REASON: str = "internalError"
INTERNAL_STATUS: int = 500
INTERNAL_LIMIT: int = 5
ROLLED_BACK_GUARD: str = (
    r"\b(?:CREATE|ALTER|DROP|UNDROP|TRUNCATE|GRANT|REVOKE|EXPORT|LOAD|INFORMATION_SCHEMA|"
    r"EXTERNAL_QUERY|COMMIT|ROLLBACK|BEGIN)\b|region-"
)

FIXTURE_STATEMENT: str = (
    r"^CREATE (?:TABLE|FUNCTION|PROCEDURE|PROPERTY GRAPH) SQLQUACK_DATASET\.([a-z][a-z0-9_]*) "
)

STOP_REASONS: frozenset[str] = frozenset(
    {
        "accessDenied",
        "backendError",
        "billingNotEnabled",
        "billingTierLimitExceeded",
        "bytesBilledLimitExceeded",
        "internalError",
        "quotaExceeded",
        "rateLimitExceeded",
        "resourcesExceeded",
        "responseTooLarge",
        "unauthorized",
    }
)
RETRIED_REASONS: frozenset[str] = frozenset({"backendError", "internalError", "rateLimitExceeded"})
RETRIED_STATUSES: frozenset[int] = frozenset({0, 429, 500, 502, 503, 504})
STOP_STATUSES: frozenset[int] = frozenset({401, 403})
ATTEMPTS: int = 6
BACKOFF_SECONDS: float = 1.0
BACKOFF_LIMIT_SECONDS: float = 32.0
DUPLICATE_STATUS: int = 409
NOT_FOUND_STATUS: int = 404
HTTP_ERROR_STATUS: int = 400
DONE_STATE: str = "DONE"

SYNTAX_CODE: str = "SYNTX"
PARSER_REASON: str = "invalidQuery"
PARSER_REFUSALS_FILE: Path = Path(__file__).parent / "parser_refusals.json"
PARSER_REFUSALS_FORMAT: str = "sqlquack-bigquery-parser-refusals/1"
PARSER_GRAMMAR_SHA256: str = "38a88ee136bd770fca435b5ba4c60a45251a91daa51710fc5d783f957971737e"
PARSER_GRAMMAR: str = (
    "https://github.com/google/googlesql.git@2026.11.1 "
    "(d48c569aebd63870ff6acecf08b6611015ef1d9d) googlesql/parser/googlesql.tm"
)
PARSER_REFUSALS_READ: str = (
    "every MakeSyntaxError call's message: its string literals in order, dynamic parts as .+?, "
    "anchored at the start; the bare 'Syntax error: ' prefixes and runtime status messages are "
    "left to the generic Syntax error: class"
)
REFUSAL_CALL: str = r"MakeSyntaxError\s*\("
REFUSAL_CODE: str = "PR{number:03d}"
LITERAL_PATTERN: str = r'R"\((.*?)\)"|"((?:[^"\\]|\\.)*)"'
STRING_BUILDER: str = "absl::Str"
STRING_BUILDER_LITERAL: str = r'\s*(R"\(|")'
DYNAMIC_PART: str = ".+?"
FORMAT_SLOT: str = "%s"
BARE_SYNTAX_PATTERNS: frozenset[str] = frozenset({"^Syntax error: ", "^Syntax error: '"})
QUOTE: str = '"'
BACKSLASH: str = "\\"
COMMA: str = ","
DEPTH_CHANGE: dict[str, int] = {"(": 1, "[": 1, "{": 1, ")": -1, "]": -1, "}": -1}
PARSER_INTERNAL_CLASSES: tuple[ErrorClass, ...] = (
    ErrorClass(
        PARSER_REASON, r'^EXCEPT must be followed by ALL, DISTINCT, or "\("', "PI001", SYNTAX_ERROR
    ),
    ErrorClass(
        PARSER_REASON,
        r"^Reserved keyword '.+?' may not be used as a label name without backticks",
        "PI002",
        SYNTAX_ERROR,
    ),
)
GENERIC_SYNTAX: ErrorClass = ErrorClass(PARSER_REASON, r"^Syntax error:", SYNTAX_CODE, SYNTAX_ERROR)
SHAPE_PATTERNS: tuple[str, ...] = (
    r"^Expected keyword ",
    r"cannot have both OR REPLACE and IF NOT EXISTS",
    r"is only allowed inside of a loop body",
    r"^INNER JOIN must have an immediately following ON or USING",
    r"in set operations cannot be used without CORRESPONDING",
    r"^CREATE INDEX is not supported",
    r"^Non-path index key expression for CREATE INDEX is not supported",
    r"^Enforcement of (?:primary|foreign) keys is not supported",
    r"^Arrays of arrays are not supported",
    r"^Positional parameters are not supported",
    r"^Parameterized types are not allowed in CAST expressions",
    r"^Statement not supported: ",
    r"^Transaction control statements are supported only in scripts",
    r"^CREATE TEMPORARY (?:TABLE )?FUNCTION statements ",
    r"^Ordinals are not allowed in ",
    r"^GROUP BY ordinal not allowed in pipe AGGREGATE",
    r"^ORDER BY ordinal is not allowed in GQL",
    r"^WHERE clause is not supported inside Window function call",
    r"^HAVING modifier is not supported on analytic functions",
    r"^ORDER BY in arguments is not supported on analytic functions",
    r"^Temporary tables may not be qualified",
    r"^Temporary function name contains a period",
    r"columns must be named, but column \d+ has no name",
    r"^Column must have a type",
    r"^Parameters in function declarations must include both name and type",
    r"^No column definitions in CREATE TABLE",
    r"^ALTER (?:RESERVATION|CAPACITY|BI_CAPACITY) statements require a path expression",
    r"^Table-valued function SQL body without a RETURNS TABLE clause",
    r"^CREATE TABLE COPY cannot be used together with ",
    r"^ALTER TABLE with schema changes does not support IF EXISTS",
    r"without FROM clause cannot ",
    r"^Analytic function not allowed in SELECT without FROM clause",
    r"^Pipe AGGREGATE cannot have both an empty aggregate list",
    r"^Argument \* can only be used in COUNT\(\*\)",
    r"^LIMIT expects an integer literal or parameter",
    r"^Missing policy id",
    r"^object_type is required, but missing",
    r"options cannot be empty",
    r"SET OPTIONS cannot be empty",
    r"^There must be at least one option specified in WITH COLUMN OPTIONS",
    r"^Unsupported empty struct type",
    r"^Primary key constraints must at least have one column",
    r"^Cannot re-raise an existing exception outside of an exception handler",
)
SHAPE_CLASSES: tuple[ErrorClass, ...] = (
    ErrorClass("invalidQuery", "|".join(SHAPE_PATTERNS), "SHAPE", SEMANTIC_ERROR),
    ErrorClass("invalid", "|".join(SHAPE_PATTERNS), "SHAPE", SEMANTIC_ERROR),
)
NOT_SYNTAX_LIKE: str = r"^(?!(?:[Uu]nexpected|[Ee]xpected)\b)(?!.*\b[Ss]yntax\b)"
QUALIFIED_CLASSES: tuple[ErrorClass, ...] = tuple(
    ErrorClass(
        reason,
        r"^(?:Table|Routine|Model|PropertyGraph) \S+ must be qualified with a dataset",
        "QUALD",
        SEMANTIC_ERROR,
    )
    for reason in ("invalid", "invalidQuery")
)
DRY_RUN_CLASSES: tuple[ErrorClass, ...] = (
    *SHAPE_CLASSES,
    *QUALIFIED_CLASSES,
    ErrorClass("invalidQuery", r"^Unrecognized name:", "UNREC", SEMANTIC_ERROR),
    ErrorClass("invalidQuery", r"^Function not found:", "FUNNF", SEMANTIC_ERROR),
    ErrorClass("invalidQuery", r"^Table-valued function not found:", "TVFNF", SEMANTIC_ERROR),
    ErrorClass("invalidQuery", r"^Type not found:", "TYPNF", SEMANTIC_ERROR),
    ErrorClass("invalidQuery", r"^No matching signature for", "NOSIG", SEMANTIC_ERROR),
    ErrorClass("invalidQuery", r"^Could not cast literal", "CASTL", SEMANTIC_ERROR),
    ErrorClass("notFound", r"^Not found:", "NOTFD", SEMANTIC_ERROR),
    ErrorClass("duplicate", r"^Already Exists:", "EXIST", SEMANTIC_ERROR),
    ErrorClass(
        "invalidQuery", r"^Expected type \S+; found \S+; failed to set ", "OPTVT", SEMANTIC_ERROR
    ),
    ErrorClass("invalidQuery", r"^Unexpected TVF argument type found", "TVFAR", SEMANTIC_ERROR),
    ErrorClass("invalidQuery", NOT_SYNTAX_LIKE, "ANLYS", SEMANTIC_ERROR),
    ErrorClass("invalid", NOT_SYNTAX_LIKE, "INVLD", SEMANTIC_ERROR),
)
EXECUTE_CLASSES: tuple[ErrorClass, ...] = (
    *SHAPE_CLASSES,
    *QUALIFIED_CLASSES,
    ErrorClass("invalidQuery", r"^Unrecognized name:", "UNREC", SEMANTIC_ERROR),
    ErrorClass("invalidQuery", r"^System variable not found:", "SYSVR", SEMANTIC_ERROR),
    ErrorClass("notFound", r"^Not found:", "NOTFD", SEMANTIC_ERROR),
    ErrorClass("invalidQuery", NOT_SYNTAX_LIKE, "RUNTM", RUNTIME_ERROR),
)
CASE_DATASET_CLASSES: tuple[ErrorClass, ...] = (
    *EXECUTE_CLASSES,
    ErrorClass("duplicate", r"^Already Exists:", "EXIST", SEMANTIC_ERROR),
    ErrorClass("invalid", NOT_SYNTAX_LIKE, "INVLD", SEMANTIC_ERROR),
)
TIMED_CLASSES: tuple[ErrorClass, ...] = (
    ErrorClass(
        "stopped",
        r"^Job execution was cancelled: Job timed out after \d+ sec$",
        "TIMEO",
        RUNTIME_ERROR,
    ),
    ErrorClass(
        "stopped",
        r"^Job execution was cancelled: User requested cancellation$",
        "CANCL",
        RUNTIME_ERROR,
    ),
    *EXECUTE_CLASSES,
)

DATASET_OPTIONS: tuple[str, ...] = (
    "defaultCollation",
    "defaultRoundingMode",
    "defaultTableExpirationMs",
    "description",
    "friendlyName",
    "isCaseInsensitive",
    "labels",
    "maxTimeTravelHours",
)
MISSING: str = "missing"
BYTES_KEY: str = "bytes"
BYTE_KEYS: tuple[str, ...] = (
    "dry_run_bytes_processed",
    "dry_run_bytes_billed",
    "executed_bytes_processed",
    "executed_bytes_billed",
)
LIST_LIMIT: str = "1000"
START_STATE_KEY: str = "start_state"
END_STATE_KEY: str = "end_state"
REFEREE_COMMIT_KEY: str = "referee"
REFEREE_DIRTY: str = "+uncommitted"
REFEREE_PATHS: tuple[str, ...] = ("scripts/bigquery_receipts", "scripts/run_bigquery_receipts.py")
GIT_HEAD: tuple[str, ...] = ("git", "rev-parse", "HEAD")
GIT_STATUS: tuple[str, ...] = ("git", "status", "--porcelain", "--untracked-files=no")

COMPARED_KEYS: tuple[str, ...] = ("phase", "engine_code", "parse_only_sqlstate", "capture_method")
COMMAND_RUN: str = "run"
COMMAND_COMPARE: str = "compare"
COMMAND_PROBE: str = "probe"
COMMAND_REFUSALS: str = "refusals"
JOBS_KEY: str = "jobs"
SHOWN_DIFFERENCES: int = 20
UNMAPPED_HEAD: int = 60
