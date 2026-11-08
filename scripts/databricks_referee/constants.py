"""The Databricks referee's fixed names: formats, capture, phases and the error-class table."""

from __future__ import annotations

import re

from scripts.databricks_referee.models import SnapshotKind

ENGINE: str = "databricks"
ENGINE_VERSION: str = "2026.44"
RECEIPT_FORMAT: str = "sqlquack-syntax-receipt/2"
RUN_AUDIT_FORMAT: str = "sqlquack-databricks-run-audit/1"
CAPTURE: str = "execute_autocommit_in_throwaway_database"
PARSER_CHECK: str = "whole_text"
RUN_PREFIX: str = "databricks-2026.44-"
CATALOG: str = "workspace"
SCHEMA_PREFIX: str = "sqlquack_referee_"

ENV_HOST: str = "DATABRICKS_HOST"
ENV_WAREHOUSE: str = "DATABRICKS_WAREHOUSE_ID"
# Optional: a case deadline other than CASE_SECONDS, for a retry of cases that ran to it.
ENV_CASE_SECONDS: str = "DATABRICKS_REFEREE_CASE_SECONDS"
ENV_CLIENT_ID: str = "DATABRICKS_CLIENT_ID"
ENV_CLIENT_SECRET: str = "DATABRICKS_CLIENT_SECRET"
TOKEN_PATH: str = "/oidc/v1/token"
STATEMENTS_PATH: str = "/api/2.0/sql/statements"
TOKEN_SCOPE: str = "all-apis"
TOKEN_REFRESH_SECONDS: float = 1800.0
# The warehouse, which a check before each case reads to prove the token still works.
WAREHOUSES_PATH: str = "/api/2.0/sql/warehouses"
# The warehouse's query history, which names the statements that ran in a time window.
HISTORY_PATH: str = "/api/2.0/sql/history/queries"
HISTORY_FAILED: str = "FAILED"
HISTORY_PAGE: int = 100
# The HTTP codes and body the workspace rejects an expired or revoked token with.
TOKEN_REJECTED_CODES: frozenset[int] = frozenset({401, 403})
TOKEN_REJECTED: re.Pattern[str] = re.compile(r"invalid (?:access )?token", re.I)
WAIT_TIMEOUT: str = "30s"
POLL_SECONDS: float = 1.0
CASE_SECONDS: float = 300.0
HTTP_SECONDS: float = 120.0
HTTP_ATTEMPTS: int = 4
HTTP_PAUSE_SECONDS: float = 5.0
WORKERS: int = 16

STATE_SUCCEEDED: str = "SUCCEEDED"
STATE_FAILED: str = "FAILED"
STATES_PENDING: frozenset[str] = frozenset({"PENDING", "RUNNING"})
STATES_STOPPED: frozenset[str] = frozenset({"CANCELED", "CLOSED"})
VERSION_QUERY: str = "SELECT current_version()"
# current_version() is asked at most once a minute and once more after the last case; a run
# whose answers differ writes no receipt, so each receipt's release is the one that answered it.
VERSION_SECONDS: float = 60.0
# A case schema's DROP ... IF EXISTS CASCADE is asked up to this many times.
DROP_ATTEMPTS: int = 3

ACCEPTED: str = "accepted"
SYNTAX_ERROR: str = "syntax_error"
SEMANTIC_ERROR: str = "semantic_error"
RUNTIME_ERROR: str = "runtime_error"
UNSUPPORTED_FEATURE: str = "unsupported_feature"
OUTCOME_ACCEPTED: str = "accepted"
OUTCOME_ERROR: str = "error"
BASIS_RAN: str = "the statement ran to completion"
BASIS_PARSER: str = (
    "the parser raised {error_class} (the message carries the parser's == SQL == excerpt)"
)
BASIS_LATER: str = "the parsed statement failed with {error_class}"

# The excerpt Spark's ParseException appends, in its older and newer header; only an error the
# parser or the parse-tree builder raised carries it.
PARSER_MARK: re.Pattern[str] = re.compile(r"\n== SQL(?: \(line \d+, position \d+\))? ==")
ERROR_CLASS: re.Pattern[str] = re.compile(r"^\s*\[([A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*)\]")
# A Unity Catalog service error: its class follows a per-request id, which receipts mask.
SERVICE_ERROR_CLASS: re.Pattern[str] = re.compile(
    r"^\s*\[RequestId=[0-9a-f-]+ ErrorClass=([A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*)\]"
)
REQUEST_ID: re.Pattern[str] = re.compile(r"RequestId=[0-9a-f-]+")
REQUEST_ID_MASK: str = "RequestId=<id>"
# The per-case schema a message quotes: its run stem and case number differ between runs.
CASE_SCHEMA: re.Pattern[str] = re.compile(r"\bsqlquack_referee_[0-9a-z_]+")
CASE_SCHEMA_MASK: str = "sqlquack_referee_<run>"
SQLSTATE_OF: re.Pattern[str] = re.compile(r"SQLSTATE: ([0-9A-Z]{5})")
URL: re.Pattern[str] = re.compile(r"\b[a-z][a-z0-9+.-]*://\S+")
URL_MASK: str = "<url>"
ADDRESS: re.Pattern[str] = re.compile(r"(?<=\w)@(?=\w)")
ADDRESS_MASK: str = " at "
# Names of the account and its workspace that a message may quote: the account id, the metastore,
# the referee's principal and the workspace's storage bucket. Receipts are public, so each is
# masked; none is part of the statement the case sent.
ENVIRONMENT_MASKS: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r"\baccount [0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}\b"), "account <account>"),
    (re.compile(r"\bMetastore '[^']*'"), "Metastore '<metastore>'"),
    (re.compile(r"`[^`]*` of type METASTORE\b"), "`<metastore>` of type METASTORE"),
    (re.compile(r"\bPrincipal '[^']*'"), "Principal '<principal>'"),
    (re.compile(r"\bstorage bucket '[^']*'"), "storage bucket '<bucket>'"),
)

# The rule: a refusal whose message carries the parser's == SQL == excerpt was raised by the
# parser or the parse-tree builder, and is a syntax_error, whatever its class or SQLSTATE says.
# Each class the parser raises is reviewed here by its full class and subclass; any other parser
# class stops the run, so a new one is reviewed before it is counted.
PARSER_CLASSES: frozenset[str] = frozenset(
    {
        "PARSE_SYNTAX_ERROR",
        "PARSE_EMPTY_STATEMENT",
        "UNCLOSED_BRACKETED_COMMENT",
        # The parse-tree builder refuses a statement the grammar admits.
        "CREATE_VIEW_WITH_IF_NOT_EXISTS_AND_REPLACE",
        "DUPLICATE_KEY",
        "MATCH_RECOGNIZE_MEASURES_MUST_BE_ALIASED",
        "MERGE_WITHOUT_WHEN",
        "MISSING_CLAUSES_FOR_OPERATION",
        "TEMP_TABLE_CREATION_MUTUAL_EXCLUSIVE_SPECS.WITH_PERMANENT_TABLE_SUGGESTION",
        "UNSUPPORTED_CLAUSE_FOR_OPERATION",
        "UNSUPPORTED_DATATYPE",
        "UNSUPPORTED_FEATURE.TEMPORARY_VIEW_WITH_SCHEMA_BINDING_MODE",
        "UNSUPPORTED_TIMESERIES_WITH_MORE_THAN_ONE_COLUMN",
        # Databricks' parser refuses a sample fraction outside [0, 100] and IF NOT EXISTS on a
        # temporary view; both classes are the warehouse's own (Spark 4.2.0 has neither), and
        # their receipts carry the parser's excerpt.
        "INVALID_TABLESAMPLE_FRACTION",
        "TEMP_VIEW_IF_NOT_EXISTS_NOT_ALLOWED",
        # The parse-tree builder checks a utility statement's operands and refuses them with the
        # excerpt: SET PATH's schema names, SET QUERY_TAGS' form, SYNC's matching table names, and
        # DESC ... AS JSON on one column.
        "INVALID_SQL_PATH_SCHEMA_REFERENCE",
        "SET_QUERY_TAGS_SYNTAX_INVALID",
        "SYNC_SRC_TARGET_TBL_NOT_SAME",
        "UNSUPPORTED_FEATURE.DESC_TABLE_COLUMN_JSON",
        # The parser refuses a statement's form or a literal's text while building the plan:
        # a SET, an empty IN list, a REFRESH path, a SHOW FUNCTIONS scope, a variable with neither
        # type nor default, ANALYZE ... DELTA STATISTICS FOR, DROP INDEX, a staging path, a typed
        # literal's value or type, and an AUTO CDC sequence clause without its storage clause.
        "AUTO_CDC_BITEMPORAL_REQUIRES_SYSTEM_SEQUENCE_BY",
        "AUTO_CDC_SYSTEM_SEQUENCE_BY_REQUIRES_BITEMPORAL",
        "INVALID_SET_SYNTAX",
        "INVALID_SQL_SYNTAX.ANALYZE_TABLE_DELTA_STATS_UNEXPECTED_TOKEN",
        "INVALID_SQL_SYNTAX.EMPTY_IN_PREDICATE",
        "INVALID_SQL_SYNTAX.EMPTY_QUANTIFIED_PATTERN",
        "INVALID_SQL_SYNTAX.EMPTY_REFRESH_RESOURCE_PATH",
        "INVALID_SQL_SYNTAX.INVALID_REFRESH_RESOURCE_PATH",
        "INVALID_SQL_SYNTAX.SHOW_FUNCTIONS_INVALID_SCOPE",
        "INVALID_SQL_SYNTAX.VARIABLE_TYPE_OR_DEFAULT_REQUIRED",
        "INVALID_STAGING_PATH_IN_STAGING_ACCESS_QUERY",
        "INVALID_STATEMENT_OR_CLAUSE",
        "INVALID_TYPED_LITERAL",
        "UNSUPPORTED_TYPED_LITERAL",
    }
)
# Spark also writes the excerpt, with its line and position, under an error raised while the
# parsed statement runs (an ANSI runtime error of an expression). Each such class is reviewed here
# by its full class and subclass and is a runtime_error, never a syntax_error; any other
# excerpt-bearing class outside PARSER_CLASSES stops the run.
EXCERPT_RUNTIME_CLASSES: frozenset[str] = frozenset(
    {
        "CAST_INVALID_INPUT",
        "DIVIDE_BY_ZERO",
        "INVALID_ARRAY_INDEX_IN_ELEMENT_AT",
    }
)
# A class the parse-tree builder raises, with the excerpt, for a feature the workspace has not
# enabled: the text parsed, so the refusal is unsupported_feature, never syntax_error.
EXCERPT_UNSUPPORTED_CLASSES: frozenset[str] = frozenset({"UC_SECRETS_NOT_ENABLED"})
# The engine code a reviewed parser message without a SQLSTATE is recorded with, so that the
# receipt's syntax_error carries a code dialect.toml's syntax_codes lists; the receipt's sqlstate
# stays empty, as the engine sent it.
PARSER_MESSAGE_CODE: str = "PARSE"
# Reviewed phase of each error class raised after parsing, by its full class and subclass; any
# other class stops the run. None is syntax_error: only the parser decides syntax.
LATER_PHASES: dict[str, str] = {
    "AMBIGUOUS_REFERENCE": SEMANTIC_ERROR,
    "CANNOT_RESOLVE_STAR_EXPAND": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.UNEXPECTED_INPUT_TYPE": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.CAST_WITH_CONF_SUGGESTION": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.CAST_WITH_FUNC_SUGGESTION": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.INVALID_JSON_SCHEMA": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.UNSUPPORTED_INPUT_TYPE": SEMANTIC_ERROR,
    "MISSING_PARAMETER_FOR_KAFKA": SEMANTIC_ERROR,
    "DELTA_INSERT_COLUMN_ARITY_MISMATCH": SEMANTIC_ERROR,
    "DELTA_MERGE_UNRESOLVED_EXPRESSION": SEMANTIC_ERROR,
    "DELTA_MISSING_COLUMN": SEMANTIC_ERROR,
    "INSERT_COLUMN_ARITY_MISMATCH.NOT_ENOUGH_DATA_COLUMNS": SEMANTIC_ERROR,
    "INVALID_LIMIT_LIKE_EXPRESSION.DATA_TYPE": SEMANTIC_ERROR,
    "INVALID_USAGE_OF_STAR_OR_REGEX": SEMANTIC_ERROR,
    "MATCH_RECOGNIZE_EMPTY_MEASURES": SEMANTIC_ERROR,
    "NUM_COLUMNS_MISMATCH": SEMANTIC_ERROR,
    "ORDER_BY_POS_OUT_OF_RANGE": SEMANTIC_ERROR,
    "TABLE_OR_VIEW_NOT_FOUND": SEMANTIC_ERROR,
    "UNRESOLVABLE_TABLE_VALUED_FUNCTION": SEMANTIC_ERROR,
    "UNRESOLVED_COLUMN.WITH_SUGGESTION": SEMANTIC_ERROR,
    "UNRESOLVED_COLUMN.WITHOUT_SUGGESTION": SEMANTIC_ERROR,
    "UNRESOLVED_INSERT_REPLACE_USING_COLUMN": SEMANTIC_ERROR,
    "UNRESOLVED_ROUTINE": SEMANTIC_ERROR,
    "UNSUPPORTED_GROUPING_EXPRESSION": SEMANTIC_ERROR,
    "WRONG_NUM_ARGS.WITHOUT_SUGGESTION": SEMANTIC_ERROR,
    "WRONG_NUM_ARGS.WITH_SUGGESTION": SEMANTIC_ERROR,
    "REQUIRED_PARAMETER_NOT_FOUND": SEMANTIC_ERROR,
    # Analysis or the catalog refuses a name, a missing object or the state of the setup's objects.
    "ALL_PARTITION_COLUMNS_NOT_ALLOWED": SEMANTIC_ERROR,
    "COLLATION_INVALID_NAME": SEMANTIC_ERROR,
    "COLUMN_NOT_FOUND_IN_TABLE": SEMANTIC_ERROR,
    "COLUMN_NOT_FOUND_IN_SCHEMA": SEMANTIC_ERROR,
    "CREATE_VIEW_COLUMN_ARITY_MISMATCH.TOO_MANY_DATA_COLUMNS": SEMANTIC_ERROR,
    "DATA_SOURCE_NOT_FOUND": SEMANTIC_ERROR,
    "DELTA_INVALID_CHARACTERS_IN_COLUMN_NAMES": SEMANTIC_ERROR,
    "FIELD_NOT_FOUND": SEMANTIC_ERROR,
    "DELTA_ALTER_TABLE_REPLACE_PARTITIONED_BY_ON_UNPARTITIONED_TABLE_NOT_ALLOWED": SEMANTIC_ERROR,
    "DELTA_CANNOT_REPLACE_MISSING_TABLE": SEMANTIC_ERROR,
    "DELTA_IDENTITY_COLUMNS_ALTER_NON_IDENTITY_COLUMN": SEMANTIC_ERROR,
    "DELTA_PATH_NOT_SPECIFIED": SEMANTIC_ERROR,
    "DROP_COMMAND_TYPE_MISMATCH": SEMANTIC_ERROR,
    "FIELD_ALREADY_EXISTS": SEMANTIC_ERROR,
    "INVALID_ATTRIBUTE_NAME_SYNTAX": SEMANTIC_ERROR,
    "MISSING_CLAUSES_FOR_OPERATION": SEMANTIC_ERROR,
    "NO_PRIMARY_KEY_CONSTRAINT_EXCEPTION": SEMANTIC_ERROR,
    "NO_SUCH_CONSTRAINT_EXCEPTION": SEMANTIC_ERROR,
    "ROUTINE_NOT_FOUND": SEMANTIC_ERROR,
    "SPECIFY_PARTITION_IS_NOT_ALLOWED": SEMANTIC_ERROR,
    "WRONG_COMMAND_FOR_OBJECT_TYPE": SEMANTIC_ERROR,
    # Unity Catalog refuses a value, principal or object after the statement parsed.
    "INVALID_PARAMETER_VALUE": SEMANTIC_ERROR,
    "INVALID_PARAMETER_VALUE.INVALID_FIELD": SEMANTIC_ERROR,
    "INVALID_PARAMETER_VALUE.INVALID_PARAMETER_VALUE": SEMANTIC_ERROR,
    "NOT_FOUND.UC_REFERENTIAL_CONSTRAINT_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "PRINCIPAL_DOES_NOT_EXIST.PRINCIPAL_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "TABLE_DOES_NOT_EXIST.RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    # The warehouse refuses what the statement asks for, not how it is written.
    "NOT_SUPPORTED_WITH_DB_SQL": UNSUPPORTED_FEATURE,
    # A function the workspace refuses for the object it reads or for Unity Catalog.
    "EVENT_LOG_UNSUPPORTED_TABLE_TYPE": UNSUPPORTED_FEATURE,
    "UC_COMMAND_NOT_SUPPORTED.WITH_RECOMMENDATION": UNSUPPORTED_FEATURE,
    # A function whose type the workspace has not enabled.
    "FEATURE_NOT_ENABLED": UNSUPPORTED_FEATURE,
    "DELTA_ALTER_TABLE_SET_MANAGED_UNSUPPORTED_COPY_MOVE_SYNTAX": UNSUPPORTED_FEATURE,
    "DELTA_ALTER_TABLE_UNSET_MANAGED_DOES_NOT_SUPPORT_CATALOG_OWNED": UNSUPPORTED_FEATURE,
    "UNSUPPORTED_MANAGED_TABLE_CREATION": UNSUPPORTED_FEATURE,
    "WITH_CREDENTIAL": UNSUPPORTED_FEATURE,
    "DELTA_FEATURE_DROP_UNSUPPORTED_CLIENT_FEATURE": SEMANTIC_ERROR,
    "DELTA_TIMESTAMP_GREATER_THAN_COMMIT": SEMANTIC_ERROR,
    "INVALID_LIMIT_LIKE_EXPRESSION.IS_NEGATIVE": SEMANTIC_ERROR,
    "INVALID_PARAMETER_VALUE.TIMESERIES_COLUMN_INVALID_TYPE": SEMANTIC_ERROR,
    "DELTA_FEATURE_DROP_WAIT_FOR_RETENTION_PERIOD": RUNTIME_ERROR,
    "UNSUPPORTED_DATA_SOURCE_WRITE": UNSUPPORTED_FEATURE,
    "UNSUPPORTED_STREAMING_TABLE_VALUED_FUNCTION": UNSUPPORTED_FEATURE,
    "DELTA_OPERATION_NOT_ALLOWED_DETAIL": UNSUPPORTED_FEATURE,
    "DELTA_TRUNCATE_TABLE_PARTITION_NOT_SUPPORTED": UNSUPPORTED_FEATURE,
    "DELTA_CLUSTER_BY_EXPRESSIONS_NOT_SUPPORTED": UNSUPPORTED_FEATURE,
    "DELTA_UNSUPPORTED_DROP_COLUMN.ENABLE_COLUMN_MAPPING": UNSUPPORTED_FEATURE,
    "DELTA_UNSUPPORTED_RENAME_COLUMN.ENABLE_COLUMN_MAPPING": UNSUPPORTED_FEATURE,
    "FOREIGN_TABLE_CONVERSION_UNSUPPORTED.UNSUPPORTED_UC_TABLE_TYPE": UNSUPPORTED_FEATURE,
    "FOREIGN_VIEW_CONVERSION_UNSUPPORTED.UNSUPPORTED_TYPE": UNSUPPORTED_FEATURE,
    "MIGRATION_NOT_SUPPORTED": UNSUPPORTED_FEATURE,
    "MIGRATION_ROLLBACK_NOT_SUPPORTED": UNSUPPORTED_FEATURE,
    "UC_COMMAND_NOT_SUPPORTED.WITHOUT_RECOMMENDATION": UNSUPPORTED_FEATURE,
    # RESOLVE DBFS LOCATION, which only a federated Hive metastore catalog supports.
    "UC_COMMAND_NOT_SUPPORTED.NON_HMS_FEDERATED_ENTITY": UNSUPPORTED_FEATURE,
    "WRONG_COLUMN_DEFAULTS_FOR_DELTA_FEATURE_NOT_ENABLED": UNSUPPORTED_FEATURE,
    # The path the statement names is refused when it runs (public DBFS root disabled).
    "DBFS_DISABLED": RUNTIME_ERROR,
    # Analysis or the catalog refuses a name, a type or a use of a function in a non-core case.
    "CATALOG_DOES_NOT_EXIST.RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "COLUMN_NOT_FOUND": SEMANTIC_ERROR,
    "CONFIG_NOT_AVAILABLE.WITHOUT_SUGGESTION": SEMANTIC_ERROR,
    "CONNECTION_NOT_FOUND": SEMANTIC_ERROR,
    "CURSOR_NOT_FOUND": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.BINARY_OP_DIFF_TYPES": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.BINARY_OP_WRONG_TYPE": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.CAST_WITHOUT_SUGGESTION": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.MAP_CONCAT_DIFF_TYPES": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.NUMERIC_INPUT_WITH_PATTERN": SEMANTIC_ERROR,
    "DATATYPE_MISMATCH.VALUE_OUT_OF_RANGE": SEMANTIC_ERROR,
    "DELTA_BLOOM_FILTER_DROP_ON_NON_EXISTING_COLUMNS": SEMANTIC_ERROR,
    "DUPLICATE_ROUTINE_PARAMETER_ASSIGNMENT.DOUBLE_NAMED_ARGUMENT_REFERENCE": SEMANTIC_ERROR,
    "INVALID_CURRENT_RECIPIENT_USAGE": SEMANTIC_ERROR,
    "INVALID_EXTRACT_BASE_FIELD_TYPE": SEMANTIC_ERROR,
    "INVALID_FORMAT.WRONG_NUM_DIGIT": SEMANTIC_ERROR,
    "INVALID_HANDLER_DECLARATION.CONDITION_NOT_FOUND": SEMANTIC_ERROR,
    "INVALID_LABEL_USAGE.DOES_NOT_EXIST": SEMANTIC_ERROR,
    "INVALID_LAMBDA_FUNCTION_CALL.LAMBDA_IN_PROJECT_FORBIDDEN": SEMANTIC_ERROR,
    "MATCH_RECOGNIZE_FUNCTION_OUTSIDE_MATCH_RECOGNIZE": SEMANTIC_ERROR,
    "NAMED_PARAMETERS_NOT_SUPPORTED": SEMANTIC_ERROR,
    "NO_SUCH_CATALOG_EXCEPTION": SEMANTIC_ERROR,
    "NO_SUCH_CREDENTIAL_EXCEPTION": SEMANTIC_ERROR,
    "NO_SUCH_EXTERNAL_LOCATION_EXCEPTION": SEMANTIC_ERROR,
    "NO_SUCH_STORAGE_CREDENTIAL_EXCEPTION": SEMANTIC_ERROR,
    "PROVIDER_DOES_NOT_EXIST.RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "RECIPIENT_DOES_NOT_EXIST.RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "ROUTINE_PARAMETER_NOT_FOUND": SEMANTIC_ERROR,
    "SCHEMA_DOES_NOT_EXIST.RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "SCHEMA_NOT_FOUND": SEMANTIC_ERROR,
    "SHARE_DOES_NOT_EXIST.RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "UC_CATALOG_NAME_NOT_PROVIDED": SEMANTIC_ERROR,
    "UC_VOLUME_NOT_FOUND": SEMANTIC_ERROR,
    "UNBOUND_SQL_PARAMETER": SEMANTIC_ERROR,
    "UNRESOLVED_VARIABLE": SEMANTIC_ERROR,
    "VARIABLE_NOT_FOUND": SEMANTIC_ERROR,
    "WINDOW_FUNCTION_WITHOUT_OVER_CLAUSE": SEMANTIC_ERROR,
    # Account- and catalog-level cases: the object a statement names is missing, already
    # exists, is not empty, or has a name or value Unity Catalog refuses.
    "CATALOG_ALREADY_EXISTS": SEMANTIC_ERROR,
    "CATALOG_NOT_EMPTY": SEMANTIC_ERROR,
    "CONNECTION_TYPE_NOT_SUPPORTED": SEMANTIC_ERROR,
    "DELTA_SHARING_INVALID_SHARED_DATA_OBJECT_NAME": SEMANTIC_ERROR,
    "INVALID_PARAMETER_VALUE.UC_TAG_POLICY_VALUE_NOT_ALLOWED": SEMANTIC_ERROR,
    "INVALID_PARAMETER_VALUE.UNSUPPORTED_LOCATION_SCHEME": SEMANTIC_ERROR,
    # A storage location given without a URI scheme, refused when Unity Catalog checks it.
    "INVALID_PARAMETER_VALUE.INVALID_LOCATION": SEMANTIC_ERROR,
    "NOT_FOUND.RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "NO_SUCH_PROVIDER_EXCEPTION": SEMANTIC_ERROR,
    "NO_SUCH_RECIPIENT_EXCEPTION": SEMANTIC_ERROR,
    "NO_SUCH_SHARE_EXCEPTION": SEMANTIC_ERROR,
    "SCHEMA_ALREADY_EXISTS": SEMANTIC_ERROR,
    "CANNOT_REFRESH_NON_MATERIALIZED_VIEW": SEMANTIC_ERROR,
    "COLUMN_ALREADY_EXISTS": SEMANTIC_ERROR,
    "MV_ST_ALTER_QUERY_INCORRECT_BACKING_TYPE": SEMANTIC_ERROR,
    "RESOURCE_DOES_NOT_EXIST.RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "STREAMING_TABLE_QUERY_INVALID": SEMANTIC_ERROR,
    "TABLE_OR_VIEW_ALREADY_EXISTS": SEMANTIC_ERROR,
    "USER_DEFINED_FUNCTIONS.SQL_TABLE_UDF_MISSING_COLUMN_NAMES": SEMANTIC_ERROR,
    # Utility, session and scripting statements refused after parsing, for the objects, columns,
    # names or values they reference.
    "DELTA_MODE_NOT_SUPPORTED": SEMANTIC_ERROR,
    "DELTA_NON_PARTITION_COLUMN_REFERENCE": SEMANTIC_ERROR,
    "DELTA_UNSUPPORTED_CLUSTERING_COLUMN_PREDICATES.NO_CLUSTERING_COLUMNS": SEMANTIC_ERROR,
    "DELTA_SHOW_PARTITION_IN_NON_PARTITIONED_TABLE": SEMANTIC_ERROR,
    "DELTA_ZORDERING_COLUMN_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "DUPLICATE_ASSIGNMENTS": SEMANTIC_ERROR,
    "DUPLICATE_VARIABLE_NAME_INSIDE_DECLARE": SEMANTIC_ERROR,
    "NOT_A_CONSTANT_STRING.WRONG_TYPE": SEMANTIC_ERROR,
    "POLICY_NOT_FOUND": SEMANTIC_ERROR,
    # The warehouse refuses an operation on the object the statement names, not its text.
    "DELTA_TABLE_ONLY_OPERATION": UNSUPPORTED_FEATURE,
    "FORBIDDEN_OPERATION": UNSUPPORTED_FEATURE,
    "INVALID_TARGET_FOR_ALTER_COMMAND": UNSUPPORTED_FEATURE,
    "MATERIALIZED_VIEW_OPERATION_NOT_ALLOWED.REPLACE_DELTA_LIVE_TABLE": UNSUPPORTED_FEATURE,
    "STREAMING_TABLE_OPERATION_NOT_ALLOWED.NON_STREAMING_TABLE": UNSUPPORTED_FEATURE,
    "STREAMING_TABLE_OPERATION_NOT_ALLOWED.UNSUPPORTED_OPERATION": UNSUPPORTED_FEATURE,
    # A value is refused when the parsed statement runs: a function's input, a setting's value,
    # or a transaction statement with no transaction open.
    "CONVERSION_INVALID_INPUT": RUNTIME_ERROR,
    "EWKT_PARSE_ERROR": RUNTIME_ERROR,
    "GEOJSON_PARSE_ERROR": RUNTIME_ERROR,
    "H3_INVALID_CELL_ID": RUNTIME_ERROR,
    "WKT_PARSE_ERROR": RUNTIME_ERROR,
    "ST_INVALID_SRID_VALUE": RUNTIME_ERROR,
    "USER_RAISED_EXCEPTION": RUNTIME_ERROR,
    # A result struct with repeated field names is refused when the result is converted to Arrow.
    "DUPLICATED_FIELD_NAME_IN_ARROW_STRUCT": RUNTIME_ERROR,
    "INVALID_CONF_VALUE.TYPE_MISMATCH": RUNTIME_ERROR,
    "INVALID_STATEMENT_TIMEOUT.NOT_AN_INTEGER": RUNTIME_ERROR,
    "NO_ACTIVE_TRANSACTION": RUNTIME_ERROR,
    # ANALYZE ... COMPUTE STORAGE METRICS reads the storage inventory when it runs.
    "ANALYZE_TABLE_COMPUTE_STORAGE_METRICS_INVENTORY_CONTENTS_NOT_VALID": RUNTIME_ERROR,
}

# Later classes the warehouse sends without a SQLSTATE: each is reviewed here, and its receipt
# carries no code. Any other class without a SQLSTATE, and every parser class, still stops the run.
LATER_WITHOUT_SQLSTATE: frozenset[str] = frozenset(
    {
        "CATALOG_NOT_EMPTY",
        "NO_SUCH_EXTERNAL_LOCATION_EXCEPTION",
        "NO_SUCH_PROVIDER_EXCEPTION",
        "NO_SUCH_RECIPIENT_EXCEPTION",
        "NO_SUCH_SHARE_EXCEPTION",
        "NO_SUCH_STORAGE_CREDENTIAL_EXCEPTION",
    }
)

# A message that names no error class, reviewed by the message itself. A parser message (one that
# carries the excerpt) is a syntax_error by the rule above; a later one has its reviewed phase.
UNCLASSED: str = "UNCLASSED"
UNCLASSED_PARSER: tuple[re.Pattern[str], ...] = (
    re.compile(r"^\S+ is not a valid byte length literal, expected syntax: "),
    re.compile(r"^TABLESAMPLE does not accept empty inputs\.$", re.M),
    re.compile(r"^Operation not allowed: "),
    re.compile(r"^Directory path and 'path' in OPTIONS should be specified one, but not both\."),
    re.compile(r"^Invalid time zone displacement value\.$", re.M),
    # REFRESH CATALOG without FOREIGN: the parser's own message, with the excerpt.
    re.compile(r"^Expected keyword 'FOREIGN'\.$", re.M),
)
UNCLASSED_LATER: tuple[tuple[re.Pattern[str], str], ...] = (
    # A Hive table's INPUTFORMAT or OUTPUTFORMAT class the warehouse cannot load: the message is
    # the class name the cases supply, and nothing else.
    (re.compile(r"^org\.example\.(?:In|Out)$"), RUNTIME_ERROR),
    (
        re.compile(r"^Unable to infer the schema\. The schema specification is required "),
        SEMANTIC_ERROR,
    ),
    (re.compile(r"^Cannot access non /Workspace local filesystem path: "), RUNTIME_ERROR),
    (re.compile(r"^QUALIFY must be used with at least one window function"), SEMANTIC_ERROR),
    (
        re.compile(r"^Streaming relations are only supported within CREATE STREAMING TABLE "),
        UNSUPPORTED_FEATURE,
    ),
    (
        re.compile(r"^Cannot create the primary key `[^`]*` because its child column\(s\) "),
        SEMANTIC_ERROR,
    ),
    (
        re.compile(
            r"^Foreign key `[^`]*` child columns and parent columns are of different sizes\."
        ),
        SEMANTIC_ERROR,
    ),
    (
        re.compile(r"^ALTER TABLE SET LOCATION does not support partition for v2 tables\."),
        UNSUPPORTED_FEATURE,
    ),
    (re.compile(r"^\S+ source does not support user-specified schema\."), UNSUPPORTED_FEATURE),
    (re.compile(r"^CREATE EXTERNAL TABLE must be accompanied by LOCATION\b"), SEMANTIC_ERROR),
    (re.compile(r"^STORED AS with file format '[^']*' is invalid\."), SEMANTIC_ERROR),
    # The warehouse resolves a LOCAL directory's path when it writes (file: with no path).
    (re.compile(r"^java\.net\.URISyntaxException: "), RUNTIME_ERROR),
    # Non-core cases: analysis refuses an AUTO CDC flow's clauses, a securable or a share name.
    (
        re.compile(
            r"^class org\.apache\.spark\.sql\.catalyst\.expressions\.Literal cannot be cast to "
            r"class org\.apache\.spark\.sql\.catalyst\.analysis\.UnresolvedAttribute "
        ),
        SEMANTIC_ERROR,
    ),
    (
        re.compile(r"^Track History is only enabled for SCD TYPE 2 in change query\."),
        SEMANTIC_ERROR,
    ),
    (
        re.compile(r"^The securable `[^`]*` of type [A-Z]+(?: [A-Z]+)? cannot be found, "),
        SEMANTIC_ERROR,
    ),
    (
        re.compile(
            r"^\S+(?: \S+)? is an invalid name for "
            r"(?:recipient|provider|share|credential|external location|catalog|securable)\.$"
        ),
        SEMANTIC_ERROR,
    ),
    # The referee's principal may not read the metastore's policies: authorization, after parsing.
    (
        re.compile(
            r"^PERMISSION_DENIED: Principal '[^']*' does not have READ METADATA on Metastore "
        ),
        SEMANTIC_ERROR,
    ),
    # Nor is it a metastore admin, which changing a metastore policy needs: also after parsing.
    (
        re.compile(r"^PERMISSION_DENIED: User is not a metastore admin for Metastore '[^']*'\.$"),
        SEMANTIC_ERROR,
    ),
    # A recipient for open sharing, which this metastore has not enabled.
    (
        re.compile(
            r"^PERMISSION_DENIED: \nExternal Delta Sharing is not enabled on the metastore\."
        ),
        UNSUPPORTED_FEATURE,
    ),
    # A group named as a member that does not exist.
    (re.compile(r"^Groups do not exist: \S+$"), SEMANTIC_ERROR),
    # A user named as a member that does not exist.
    (re.compile(r"^Users do not exist: \S+$"), SEMANTIC_ERROR),
    # A grantee that does not exist, refused once the statement parsed.
    (
        re.compile(r"^RESOURCE_DOES_NOT_EXIST: Could not find principal with name \S+$"),
        SEMANTIC_ERROR,
    ),
    # A sharing identifier that names no metastore.
    (
        re.compile(r"^INVALID_PARAMETER_VALUE: \S+ is an invalid id for metastore\.$"),
        SEMANTIC_ERROR,
    ),
    # A securable of a type the workspace does not have, named in a grant or a policy.
    (
        re.compile(
            r"^The securable `[^`]*` of type (?:MODEL_SERVICE|EXTERNAL_METADATA) cannot be "
            r"found, and (?:MODEL_SERVICE|EXTERNAL_METADATA) is not supported in ABAC policies\."
        ),
        SEMANTIC_ERROR,
    ),
    # A statement with an optional name left out fails inside the warehouse after parsing.
    (re.compile(r"^None\.get$"), RUNTIME_ERROR),
    (re.compile(r"^Filtering (?:user|group) membership is not supported\.$"), UNSUPPORTED_FEATURE),
    (re.compile(r"^NOT_FOUND: Tag policy with name \S+ not found in account "), SEMANTIC_ERROR),
    (
        re.compile(
            r"^INVALID_PARAMETER_VALUE: credential name can not be provided with a volume path$"
        ),
        SEMANTIC_ERROR,
    ),
    (re.compile(r"^Cannot refresh built-in function \S+\.; line \d+ pos \d+$"), SEMANTIC_ERROR),
    (re.compile(r"^EvalBatch not supported for window function$"), UNSUPPORTED_FEATURE),
    (re.compile(r"^Unsupported configuration TIMEZONE="), RUNTIME_ERROR),
    (re.compile(r"^Path must be absolute: "), RUNTIME_ERROR),
    (
        re.compile(
            r"^Cannot create a file reference for any files outside of a Volume or FileSpace\."
        ),
        RUNTIME_ERROR,
    ),
)
REFEREE_CLASS: str = "REFEREE_UNDECIDED"
# The referee failures a run may end with, each leaving its case undecided with no receipt: a case
# statement cancelled at the deadline, a statement whose reply was lost, and a failed setup. Any
# other referee failure stops the run.
CANCELLED_AT_DEADLINE: re.Pattern[str] = re.compile(
    r"^statement \S+ was still running after \d+ s; cancelled, so its case is undecided$"
)
# A statement whose reply never arrived may have run, so it is not sent again.
REPLY_LOST: str = "the reply to a statement was lost, so it may have run and is not sent again"
# Why a case is undecided, as the run summary lists it; a setup failure adds the error class.
UNDECIDED_DEADLINE: str = "cancelled at the deadline"
UNDECIDED_REPLY_LOST: str = "reply lost"
UNDECIDED_SETUP: str = "setup failed"
SETUP_ERROR_CLASS: re.Pattern[str] = re.compile(r"\[([A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*)\]")

# The referee isolates a case only inside its own schema. A case whose text changes an account-
# or catalog-level object (a catalog, a schema of the catalog, a share, a connection, a group, a
# governed tag, a metastore policy, a grant on one of them), or moves files outside a table, is
# held: the run stops before any statement runs. Each pattern is searched in the whole text,
# string literals included, so a script block or EXECUTE IMMEDIATE cannot hide one.
ACCOUNT_KINDS: str = (
    r"(?:FOREIGN\s+)?CATALOG|(?:FOREIGN\s+)?SCHEMA|DATABASE|CONNECTION|SERVER"
    r"|(?:EXTERNAL\s+)?LOCATION|(?:STORAGE\s+|SERVICE\s+)?CREDENTIAL|SHARE|RECIPIENT|PROVIDER"
    r"|(?:GOVERNED\s+)?TAG|GROUP|USER|POLICY|(?:CLEAN\s+)?ROOM|EXTERNAL\s+METADATA|METASTORE|SECRET"
)
ACCOUNT_SCOPE: tuple[tuple[re.Pattern[str], str], ...] = (
    (
        re.compile(
            r"\b(?:CREATE|ALTER|DROP|UNDROP|REFRESH|SYNC|COMMENT\s+ON)"
            r"(?:\s+(?:OR\s+REPLACE|OR\s+REFRESH|IF\s+NOT\s+EXISTS|IF\s+EXISTS))*"
            rf"\s+(?:{ACCOUNT_KINDS})\b",
            re.I,
        ),
        "changes an account- or catalog-level object",
    ),
    (
        re.compile(rf"\b(?:SET|UNSET)\s+TAGS?\s+ON\s+(?:{ACCOUNT_KINDS})\b", re.I),
        "tags an account- or catalog-level object",
    ),
    (
        re.compile(
            r"\b(?:GRANT|REVOKE|DENY)\b[^;]*?\bON\s+(?:ANY\s+FILE|ANONYMOUS\s+FUNCTION|EXTERNAL|"
            rf"METADATA|{ACCOUNT_KINDS})\b",
            re.I,
        ),
        "grants on an account- or catalog-level object",
    ),
    (
        re.compile(r"\b(?:GRANT|REVOKE|DENY)\b[^;]*?\b(?:TO|FROM)\s+RECIPIENT\b", re.I),
        "grants to a share recipient",
    ),
    (
        re.compile(r"^\s*(?:PUT|GET|REMOVE)\b", re.I),
        "moves files outside the case schema",
    ),
)
# The statement words a case may begin with: each runs inside the case schema unless a pattern
# above holds it. A case beginning with another word is held, since the referee cannot place it;
# a case beginning with a label, a literal, a parenthesis or an identifier the parser refuses is
# kept, because the patterns above are searched in its whole text.
CASE_SCOPE_HEADS: frozenset[str] = frozenset(
    {
        "ADD",
        "ALTER",
        "ANALYZE",
        "BEGIN",
        "CACHE",
        "CALL",
        "CLEAR",
        "COMMENT",
        "COMMIT",
        "CONVERT",
        "COPY",
        "CREATE",
        "DECLARE",
        "DELETE",
        "DENY",
        "DESC",
        "DESCRIBE",
        "DROP",
        "END",
        "EXECUTE",
        "EXPLAIN",
        "FROM",
        "FSCK",
        "GENERATE",
        "GRANT",
        "INSERT",
        "INTO",
        "LIST",
        "LOAD",
        "MATERIALIZED",
        "MERGE",
        "MSCK",
        "OPTIMIZE",
        "OR",
        "OVERWRITE",
        "REFRESH",
        "REORG",
        "REPAIR",
        "REPLACE",
        "RESET",
        "RESTORE",
        "REVOKE",
        "ROLLBACK",
        "SELECT",
        "SET",
        "SHOW",
        "STREAMING",
        "SYNC",
        "TABLE",
        "TEMP",
        "TRUNCATE",
        "UNCACHE",
        "UNDROP",
        "UNION",
        "UNSET",
        "UPDATE",
        "USE",
        "VACUUM",
        "VALUES",
        "WITH",
    }
)
# A case's first word, or a label (`l1 :`) before it.
CASE_HEAD: re.Pattern[str] = re.compile(r"^[\s(]*(?:([a-z]\w*)\s*:(?!:)\s*)?([A-Za-z_]\w*)?")
# The words that start a statement; a case starting with any other word starts with a name.
STATEMENT_WORDS: re.Pattern[str] = re.compile(r"^[A-Z_]+$")

# The sequential phase (`run --sequential`) runs the cases the scope guard holds, one at a time,
# each judged in its own schema as above. Before the first case, and after each, it reads these
# kinds of account- or catalog-level object over REST, each object keyed by its name and
# fingerprinted by the fields a change alters; volatile fields are left out. The referee's own
# case schemas are not part of the state. Metastore policies are not read: the referee's
# principal may not read them (PERMISSION_DENIED ... READ METADATA on Metastore), so a case that
# changes one is not seen.
UNITY_CATALOG_PATH: str = "/api/2.1/unity-catalog"
METASTORE_PATH: str = f"{UNITY_CATALOG_PATH}/current-metastore-assignment"
GROUPS_PATH: str = "/api/2.0/preview/scim/v2/Groups"
TAG_POLICIES_PATH: str = "/api/2.1/tag-policies"
DEFAULT_SCHEMA: str = "default"
# A Unity Catalog or tag-policy list returns next_page_token until its last page; a SCIM list is
# read SCIM_PAGE objects at a time from startIndex until totalResults.
PAGING_TOKEN: str = "page_token"
PAGING_SCIM: str = "scim"
SCIM_PAGE: int = 100
UNITY_CATALOG_FIELDS: tuple[str, ...] = ("owner", "comment", "properties", "updated_at")
IN_DEFAULT: str = f"catalog_name={CATALOG}&schema_name={DEFAULT_SCHEMA}&max_results=0"
KIND_CATALOG: str = "catalog"
KIND_SCHEMA: str = "schema"
KIND_CONNECTION: str = "connection"
KIND_SHARE: str = "share"
KIND_RECIPIENT: str = "recipient"
KIND_PROVIDER: str = "provider"
KIND_EXTERNAL_LOCATION: str = "external_location"
KIND_STORAGE_CREDENTIAL: str = "storage_credential"
KIND_SERVICE_CREDENTIAL: str = "service_credential"
KIND_TAG_POLICY: str = "tag_policy"
KIND_GROUP: str = "group"
KIND_METASTORE_GRANT: str = "metastore_grant"
KIND_CATALOG_GRANT: str = "catalog_grant"
KIND_DEFAULT_TABLE: str = "default_table"
KIND_DEFAULT_FUNCTION: str = "default_function"
KIND_DEFAULT_VOLUME: str = "default_volume"
SNAPSHOT_KINDS: dict[str, SnapshotKind] = {
    KIND_CATALOG: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/catalogs?max_results=0",
        items="catalogs",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
    ),
    KIND_SCHEMA: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/schemas?catalog_name={CATALOG}&max_results=0",
        items="schemas",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
        skip_prefix=SCHEMA_PREFIX,
    ),
    KIND_CONNECTION: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/connections?max_results=0",
        items="connections",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
    ),
    KIND_SHARE: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/shares?max_results=0",
        items="shares",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
    ),
    KIND_RECIPIENT: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/recipients?max_results=0",
        items="recipients",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
    ),
    KIND_PROVIDER: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/providers?max_results=0",
        items="providers",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
    ),
    KIND_EXTERNAL_LOCATION: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/external-locations?max_results=0",
        items="external_locations",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
    ),
    KIND_STORAGE_CREDENTIAL: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/storage-credentials?max_results=0",
        items="storage_credentials",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
    ),
    # The credentials list has no purpose filter: a storage credential is the kind above.
    KIND_SERVICE_CREDENTIAL: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/credentials?max_results=0",
        items="credentials",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
        skip=("purpose", "STORAGE"),
    ),
    KIND_TAG_POLICY: SnapshotKind(
        path=TAG_POLICIES_PATH,
        items="tag_policies",
        name="tag_key",
        fields=("description", "values.name", "update_time"),
        paging=PAGING_TOKEN,
    ),
    KIND_GROUP: SnapshotKind(
        path=f"{GROUPS_PATH}?attributes=id,displayName,members",
        items="Resources",
        name="displayName",
        fields=("id", "members.value"),
        paging=PAGING_SCIM,
    ),
    KIND_METASTORE_GRANT: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/permissions/metastore/{{metastore_id}}?max_results=0",
        items="privilege_assignments",
        name="principal",
        fields=("privileges",),
        paging=PAGING_TOKEN,
    ),
    KIND_CATALOG_GRANT: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/permissions/catalog/{CATALOG}?max_results=0",
        items="privilege_assignments",
        name="principal",
        fields=("privileges",),
        paging=PAGING_TOKEN,
    ),
    KIND_DEFAULT_TABLE: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/tables?{IN_DEFAULT}",
        items="tables",
        name="name",
        fields=(*UNITY_CATALOG_FIELDS, "table_type"),
        paging=PAGING_TOKEN,
        variant="table_type",
    ),
    KIND_DEFAULT_FUNCTION: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/functions?{IN_DEFAULT}",
        items="functions",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
    ),
    KIND_DEFAULT_VOLUME: SnapshotKind(
        path=f"{UNITY_CATALOG_PATH}/volumes?{IN_DEFAULT}",
        items="volumes",
        name="name",
        fields=UNITY_CATALOG_FIELDS,
        paging=PAGING_TOKEN,
    ),
}
# The names the cases give the objects they create, lowercased as Unity Catalog stores them: a
# fixture name, its backquoted `<name> Q` spelling, and the reserved word `SELECT`. Only an
# object a case added under one of these names is dropped; any other added object stops the run.
FIXTURE_NAMES: frozenset[str] = frozenset(
    {
        "v1",
        "v3",
        "s1",
        "sh1",
        "t1",
        "t2",
        "t3",
        "c1",
        "m1",
        "mv1",
        "st1",
        "u1",
        "g1",
        "abs",
        "select",
        "v1 q",
        "v3 q",
        "s1 q",
        "t1 q",
        "t2 q",
        "t3 q",
        "c1 q",
        "abs q",
    }
)
# The reviewed drop of each kind (and, for workspace.default's tables, each table_type), in the
# order cleanup runs them, dependents first; `{name}` is the backquoted name. A kind or table
# type with no row here has no reviewed drop, and an object of it a case added stops the run.
DROP_BY_SCIM: str = "SCIM DELETE"
DROPS: tuple[tuple[str, str | None, str], ...] = (
    (KIND_SHARE, None, "DROP SHARE IF EXISTS {name}"),
    (KIND_RECIPIENT, None, "DROP RECIPIENT IF EXISTS {name}"),
    (KIND_PROVIDER, None, "DROP PROVIDER IF EXISTS {name}"),
    (KIND_CATALOG, None, "DROP CATALOG IF EXISTS {name} CASCADE"),
    (KIND_SCHEMA, None, f"DROP SCHEMA IF EXISTS {CATALOG}.{{name}} CASCADE"),
    (KIND_DEFAULT_TABLE, "VIEW", f"DROP VIEW IF EXISTS {CATALOG}.{DEFAULT_SCHEMA}.{{name}}"),
    (KIND_DEFAULT_TABLE, "MANAGED", f"DROP TABLE IF EXISTS {CATALOG}.{DEFAULT_SCHEMA}.{{name}}"),
    (KIND_DEFAULT_TABLE, "EXTERNAL", f"DROP TABLE IF EXISTS {CATALOG}.{DEFAULT_SCHEMA}.{{name}}"),
    (KIND_DEFAULT_FUNCTION, None, f"DROP FUNCTION IF EXISTS {CATALOG}.{DEFAULT_SCHEMA}.{{name}}"),
    (KIND_DEFAULT_VOLUME, None, f"DROP VOLUME IF EXISTS {CATALOG}.{DEFAULT_SCHEMA}.{{name}}"),
    (KIND_EXTERNAL_LOCATION, None, "DROP EXTERNAL LOCATION IF EXISTS {name} FORCE"),
    (KIND_CONNECTION, None, "DROP CONNECTION IF EXISTS {name}"),
    (KIND_STORAGE_CREDENTIAL, None, "DROP STORAGE CREDENTIAL IF EXISTS {name} FORCE"),
    (KIND_SERVICE_CREDENTIAL, None, "DROP SERVICE CREDENTIAL IF EXISTS {name}"),
    (KIND_TAG_POLICY, None, "DROP GOVERNED TAG {name}"),
    (KIND_GROUP, None, DROP_BY_SCIM),
)

# Metastore policies are the one kind the snapshot cannot read. After every case that names a
# policy on the metastore, and once more after the last case, the sequential phase drops each
# fixture-named metastore policy (the grammar has no IF EXISTS here). An answer that no such
# policy exists, or that the referee is not a metastore admin (so it could not have created one),
# leaves the state as it was; a drop that succeeds, or any other answer, stops the run.
METASTORE_POLICY_CASE: re.Pattern[str] = re.compile(r"\bPOLICY\b[^;]*\bON\s+METASTORE\b", re.I)
METASTORE_POLICY_NAMES: tuple[str, ...] = ("v1", "v1 Q", "SELECT")
METASTORE_POLICY_DROP: str = "DROP POLICY {name} ON METASTORE"
# A sequential run's journal: the baseline, then each case as it starts and once it is settled.
JOURNAL_BASELINE: str = "baseline"
JOURNAL_PENDING: str = "pending"
JOURNAL_SETTLED: str = "settled"
JOURNAL_FINISHED: str = "finished"
# What a settled case left behind: the account state equal to the baseline again.
JOURNAL_RESTORED: str = "baseline restored"
# A settled case's answer: an observation, or a failure no reviewed row maps.
JOURNAL_OBSERVATION: str = "observation"
JOURNAL_FAILURE: str = "failure"
# The names a statement writes: backquoted identifiers and bare words.
STATEMENT_NAMES: re.Pattern[str] = re.compile(r"`((?:[^`]|``)+)`|\b([A-Za-z_][A-Za-z0-9_]*)\b")
METASTORE_POLICY_ABSENT: re.Pattern[str] = re.compile(
    r"^\s*(?:PERMISSION_DENIED: User is not a metastore admin for Metastore "
    r"|\[(?:RequestId=[0-9a-f-]+ ErrorClass=)?[A-Z_.]*(?:NOT_FOUND|DOES_NOT_EXIST)[A-Z_.]*\]"
    # No ABAC policy can be attached to the metastore, so none could have been created.
    r"|The securable `[^`]*` of type METASTORE cannot be found, and METASTORE is not supported "
    r"in ABAC policies\.)"
)

COMPARED_KEYS: tuple[str, ...] = ("phase", "engine_code", "parse_only_sqlstate", "capture_method")
# Cases a repeat compares by phase alone, each with the reason: the warehouse gives two answers.
PHASE_ONLY_CASES: dict[str, str] = {
    "db.ddl.alter-table.alter-table-set-managed.k9dd5015a9cae": (
        "UNSET MANAGED on a new table answers MIGRATION_ROLLBACK_NOT_SUPPORTED (42809) or, once "
        "in seven runs, DELTA_ALTER_TABLE_UNSET_MANAGED_DOES_NOT_SUPPORT_CATALOG_OWNED (0AKDC); "
        "both are unsupported_feature"
    ),
    "db.utility.optimize.k8b814db72656": (
        "OPTIMIZE FULL WHERE on a new table answers DELTA_NON_PARTITION_COLUMN_REFERENCE (42P10) "
        "or DELTA_UNSUPPORTED_CLUSTERING_COLUMN_PREDICATES.NO_CLUSTERING_COLUMNS (0A000); both "
        "are semantic_error"
    ),
    "db.utility.reorg-table.ka36dc5105ea3": (
        "REORG TABLE WHERE on a new table answers DELTA_NON_PARTITION_COLUMN_REFERENCE (42P10) "
        "or DELTA_UNSUPPORTED_CLUSTERING_COLUMN_PREDICATES.NO_CLUSTERING_COLUMNS (0A000); both "
        "are semantic_error"
    ),
}
COMMAND_RUN: str = "run"
COMMAND_COMPARE: str = "compare"
COMMAND_RECOVER: str = "recover"
COMMAND_RESUME: str = "resume"
COMMAND_REFINISH: str = "refinish"
SHOWN_DIFFERENCES: int = 20
