"""The Databricks referee's fixed names: formats, capture, phases and the error-class table."""

from __future__ import annotations

import re

from scripts.databricks_receipts.models import SnapshotKind

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
ENV_CASE_SECONDS: str = "DATABRICKS_REFEREE_CASE_SECONDS"
ENV_CLIENT_ID: str = "DATABRICKS_CLIENT_ID"
ENV_CLIENT_SECRET: str = "DATABRICKS_CLIENT_SECRET"
TOKEN_PATH: str = "/oidc/v1/token"
STATEMENTS_PATH: str = "/api/2.0/sql/statements"
TOKEN_SCOPE: str = "all-apis"
TOKEN_REFRESH_SECONDS: float = 1800.0
WAREHOUSES_PATH: str = "/api/2.0/sql/warehouses"
HISTORY_PATH: str = "/api/2.0/sql/history/queries"
HISTORY_FAILED: str = "FAILED"
HISTORY_PAGE: int = 100
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
VERSION_SECONDS: float = 60.0
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

PARSER_MARK: re.Pattern[str] = re.compile(r"\n== SQL(?: \(line \d+, position \d+\))? ==")
ERROR_CLASS: re.Pattern[str] = re.compile(r"^\s*\[([A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*)\]")
SERVICE_ERROR_CLASS: re.Pattern[str] = re.compile(
    r"^\s*\[RequestId=[0-9a-f-]+ ErrorClass=([A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*)\]"
)
REQUEST_ID: re.Pattern[str] = re.compile(r"RequestId=[0-9a-f-]+")
REQUEST_ID_MASK: str = "RequestId=<id>"
CASE_SCHEMA: re.Pattern[str] = re.compile(r"\bsqlquack_referee_[0-9a-z_]+")
CASE_SCHEMA_MASK: str = "sqlquack_referee_<run>"
SQLSTATE_OF: re.Pattern[str] = re.compile(r"SQLSTATE: ([0-9A-Z]{5})")
URL: re.Pattern[str] = re.compile(r"\b[a-z][a-z0-9+.-]*://\S+")
URL_MASK: str = "<url>"
ADDRESS: re.Pattern[str] = re.compile(r"(?<=\w)@(?=\w)")
ADDRESS_MASK: str = " at "
ENVIRONMENT_MASKS: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r"\baccount [0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}\b"), "account <account>"),
    (re.compile(r"\bMetastore '[^']*'"), "Metastore '<metastore>'"),
    (re.compile(r"`[^`]*` of type METASTORE\b"), "`<metastore>` of type METASTORE"),
    (re.compile(r"\bPrincipal '[^']*'"), "Principal '<principal>'"),
    (re.compile(r"\bstorage bucket '[^']*'"), "storage bucket '<bucket>'"),
)

PARSER_CLASSES: frozenset[str] = frozenset(
    {
        "PARSE_SYNTAX_ERROR",
        "PARSE_EMPTY_STATEMENT",
        "UNCLOSED_BRACKETED_COMMENT",
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
        "INVALID_TABLESAMPLE_FRACTION",
        "TEMP_VIEW_IF_NOT_EXISTS_NOT_ALLOWED",
        "INVALID_SQL_PATH_SCHEMA_REFERENCE",
        "SET_QUERY_TAGS_SYNTAX_INVALID",
        "SYNC_SRC_TARGET_TBL_NOT_SAME",
        "UNSUPPORTED_FEATURE.DESC_TABLE_COLUMN_JSON",
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
EXCERPT_RUNTIME_CLASSES: frozenset[str] = frozenset(
    {
        "CAST_INVALID_INPUT",
        "DIVIDE_BY_ZERO",
        "INVALID_ARRAY_INDEX_IN_ELEMENT_AT",
    }
)
EXCERPT_UNSUPPORTED_CLASSES: frozenset[str] = frozenset({"UC_SECRETS_NOT_ENABLED"})
PARSER_MESSAGE_CODE: str = "PARSE"
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
    "INVALID_PARAMETER_VALUE": SEMANTIC_ERROR,
    "INVALID_PARAMETER_VALUE.INVALID_FIELD": SEMANTIC_ERROR,
    "INVALID_PARAMETER_VALUE.INVALID_PARAMETER_VALUE": SEMANTIC_ERROR,
    "NOT_FOUND.UC_REFERENTIAL_CONSTRAINT_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "PRINCIPAL_DOES_NOT_EXIST.PRINCIPAL_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "TABLE_DOES_NOT_EXIST.RESOURCE_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "NOT_SUPPORTED_WITH_DB_SQL": UNSUPPORTED_FEATURE,
    "EVENT_LOG_UNSUPPORTED_TABLE_TYPE": UNSUPPORTED_FEATURE,
    "UC_COMMAND_NOT_SUPPORTED.WITH_RECOMMENDATION": UNSUPPORTED_FEATURE,
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
    "UC_COMMAND_NOT_SUPPORTED.NON_HMS_FEDERATED_ENTITY": UNSUPPORTED_FEATURE,
    "WRONG_COLUMN_DEFAULTS_FOR_DELTA_FEATURE_NOT_ENABLED": UNSUPPORTED_FEATURE,
    "DBFS_DISABLED": RUNTIME_ERROR,
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
    "CATALOG_ALREADY_EXISTS": SEMANTIC_ERROR,
    "CATALOG_NOT_EMPTY": SEMANTIC_ERROR,
    "CONNECTION_TYPE_NOT_SUPPORTED": SEMANTIC_ERROR,
    "DELTA_SHARING_INVALID_SHARED_DATA_OBJECT_NAME": SEMANTIC_ERROR,
    "INVALID_PARAMETER_VALUE.UC_TAG_POLICY_VALUE_NOT_ALLOWED": SEMANTIC_ERROR,
    "INVALID_PARAMETER_VALUE.UNSUPPORTED_LOCATION_SCHEME": SEMANTIC_ERROR,
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
    "DELTA_MODE_NOT_SUPPORTED": SEMANTIC_ERROR,
    "DELTA_NON_PARTITION_COLUMN_REFERENCE": SEMANTIC_ERROR,
    "DELTA_UNSUPPORTED_CLUSTERING_COLUMN_PREDICATES.NO_CLUSTERING_COLUMNS": SEMANTIC_ERROR,
    "DELTA_SHOW_PARTITION_IN_NON_PARTITIONED_TABLE": SEMANTIC_ERROR,
    "DELTA_ZORDERING_COLUMN_DOES_NOT_EXIST": SEMANTIC_ERROR,
    "DUPLICATE_ASSIGNMENTS": SEMANTIC_ERROR,
    "DUPLICATE_VARIABLE_NAME_INSIDE_DECLARE": SEMANTIC_ERROR,
    "NOT_A_CONSTANT_STRING.WRONG_TYPE": SEMANTIC_ERROR,
    "POLICY_NOT_FOUND": SEMANTIC_ERROR,
    "DELTA_TABLE_ONLY_OPERATION": UNSUPPORTED_FEATURE,
    "FORBIDDEN_OPERATION": UNSUPPORTED_FEATURE,
    "INVALID_TARGET_FOR_ALTER_COMMAND": UNSUPPORTED_FEATURE,
    "MATERIALIZED_VIEW_OPERATION_NOT_ALLOWED.REPLACE_DELTA_LIVE_TABLE": UNSUPPORTED_FEATURE,
    "STREAMING_TABLE_OPERATION_NOT_ALLOWED.NON_STREAMING_TABLE": UNSUPPORTED_FEATURE,
    "STREAMING_TABLE_OPERATION_NOT_ALLOWED.UNSUPPORTED_OPERATION": UNSUPPORTED_FEATURE,
    "CONVERSION_INVALID_INPUT": RUNTIME_ERROR,
    "EWKT_PARSE_ERROR": RUNTIME_ERROR,
    "GEOJSON_PARSE_ERROR": RUNTIME_ERROR,
    "H3_INVALID_CELL_ID": RUNTIME_ERROR,
    "WKT_PARSE_ERROR": RUNTIME_ERROR,
    "ST_INVALID_SRID_VALUE": RUNTIME_ERROR,
    "USER_RAISED_EXCEPTION": RUNTIME_ERROR,
    "DUPLICATED_FIELD_NAME_IN_ARROW_STRUCT": RUNTIME_ERROR,
    "INVALID_CONF_VALUE.TYPE_MISMATCH": RUNTIME_ERROR,
    "INVALID_STATEMENT_TIMEOUT.NOT_AN_INTEGER": RUNTIME_ERROR,
    "NO_ACTIVE_TRANSACTION": RUNTIME_ERROR,
    "ANALYZE_TABLE_COMPUTE_STORAGE_METRICS_INVENTORY_CONTENTS_NOT_VALID": RUNTIME_ERROR,
}

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

UNCLASSED: str = "UNCLASSED"
UNCLASSED_PARSER: tuple[re.Pattern[str], ...] = (
    re.compile(r"^\S+ is not a valid byte length literal, expected syntax: "),
    re.compile(r"^TABLESAMPLE does not accept empty inputs\.$", re.M),
    re.compile(r"^Operation not allowed: "),
    re.compile(r"^Directory path and 'path' in OPTIONS should be specified one, but not both\."),
    re.compile(r"^Invalid time zone displacement value\.$", re.M),
    re.compile(r"^Expected keyword 'FOREIGN'\.$", re.M),
)
UNCLASSED_LATER: tuple[tuple[re.Pattern[str], str], ...] = (
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
    (re.compile(r"^java\.net\.URISyntaxException: "), RUNTIME_ERROR),
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
    (
        re.compile(
            r"^PERMISSION_DENIED: Principal '[^']*' does not have READ METADATA on Metastore "
        ),
        SEMANTIC_ERROR,
    ),
    (
        re.compile(r"^PERMISSION_DENIED: User is not a metastore admin for Metastore '[^']*'\.$"),
        SEMANTIC_ERROR,
    ),
    (
        re.compile(
            r"^PERMISSION_DENIED: \nExternal Delta Sharing is not enabled on the metastore\."
        ),
        UNSUPPORTED_FEATURE,
    ),
    (re.compile(r"^Groups do not exist: \S+$"), SEMANTIC_ERROR),
    (re.compile(r"^Users do not exist: \S+$"), SEMANTIC_ERROR),
    (
        re.compile(r"^RESOURCE_DOES_NOT_EXIST: Could not find principal with name \S+$"),
        SEMANTIC_ERROR,
    ),
    (
        re.compile(r"^INVALID_PARAMETER_VALUE: \S+ is an invalid id for metastore\.$"),
        SEMANTIC_ERROR,
    ),
    (
        re.compile(
            r"^The securable `[^`]*` of type (?:MODEL_SERVICE|EXTERNAL_METADATA) cannot be "
            r"found, and (?:MODEL_SERVICE|EXTERNAL_METADATA) is not supported in ABAC policies\."
        ),
        SEMANTIC_ERROR,
    ),
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
CANCELLED_AT_DEADLINE: re.Pattern[str] = re.compile(
    r"^statement \S+ was still running after \d+ s; cancelled, so its case is undecided$"
)
REPLY_LOST: str = "the reply to a statement was lost, so it may have run and is not sent again"
UNDECIDED_DEADLINE: str = "cancelled at the deadline"
UNDECIDED_REPLY_LOST: str = "reply lost"
UNDECIDED_SETUP: str = "setup failed"
SETUP_ERROR_CLASS: re.Pattern[str] = re.compile(r"\[([A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*)\]")

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
CASE_HEAD: re.Pattern[str] = re.compile(r"^[\s(]*(?:([a-z]\w*)\s*:(?!:)\s*)?([A-Za-z_]\w*)?")
STATEMENT_WORDS: re.Pattern[str] = re.compile(r"^[A-Z_]+$")

UNITY_CATALOG_PATH: str = "/api/2.1/unity-catalog"
METASTORE_PATH: str = f"{UNITY_CATALOG_PATH}/current-metastore-assignment"
GROUPS_PATH: str = "/api/2.0/preview/scim/v2/Groups"
TAG_POLICIES_PATH: str = "/api/2.1/tag-policies"
DEFAULT_SCHEMA: str = "default"
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

METASTORE_POLICY_CASE: re.Pattern[str] = re.compile(r"\bPOLICY\b[^;]*\bON\s+METASTORE\b", re.I)
METASTORE_POLICY_NAMES: tuple[str, ...] = ("v1", "v1 Q", "SELECT")
METASTORE_POLICY_DROP: str = "DROP POLICY {name} ON METASTORE"
JOURNAL_BASELINE: str = "baseline"
JOURNAL_PENDING: str = "pending"
JOURNAL_SETTLED: str = "settled"
JOURNAL_FINISHED: str = "finished"
JOURNAL_RESTORED: str = "baseline restored"
JOURNAL_OBSERVATION: str = "observation"
JOURNAL_FAILURE: str = "failure"
STATEMENT_NAMES: re.Pattern[str] = re.compile(r"`((?:[^`]|``)+)`|\b([A-Za-z_][A-Za-z0-9_]*)\b")
METASTORE_POLICY_ABSENT: re.Pattern[str] = re.compile(
    r"^\s*(?:PERMISSION_DENIED: User is not a metastore admin for Metastore "
    r"|\[(?:RequestId=[0-9a-f-]+ ErrorClass=)?[A-Z_.]*(?:NOT_FOUND|DOES_NOT_EXIST)[A-Z_.]*\]"
    r"|The securable `[^`]*` of type METASTORE cannot be found, and METASTORE is not supported "
    r"in ABAC policies\.)"
)

COMPARED_KEYS: tuple[str, ...] = ("phase", "engine_code", "parse_only_sqlstate", "capture_method")
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
HTTP_RETRY_STATUS: int = 429
HTTP_SERVER_ERROR: int = 500
SETUP_COMPOUND_FROM: int = 2
QUERY_MARK: str = "?"
PYCACHE: str = "__pycache__"
JOURNAL_ANSWER: str = "answer"
