//! Constants of parser.

/// The text "ALGORITHM", as comparisons test it.
pub(crate) const ALGORITHM: &str = "ALGORITHM";

/// The text "ARRAY", as comparisons test it.
pub(crate) const ARRAY: &str = "ARRAY";

/// The text "ARRAY_LENGTH", as comparisons test it.
pub(crate) const ARRAY_LENGTH: &str = "ARRAY_LENGTH";

/// The text "ARRAY_SIZE", as comparisons test it.
pub(crate) const ARRAY_SIZE: &str = "ARRAY_SIZE";

/// The text "ATAN", as comparisons test it.
pub(crate) const ATAN: &str = "ATAN";

/// The text "@", as comparisons test it.
pub(crate) const AT_SIGN: &str = "@";

/// The text "BUFFER_USAGE_LIMIT", as comparisons test it.
pub(crate) const BUFFER_USAGE_LIMIT: &str = "BUFFER_USAGE_LIMIT";

/// The text "CHAR", as comparisons test it.
pub(crate) const CHAR: &str = "CHAR";

/// The text "CHARACTER", as comparisons test it.
pub(crate) const CHARACTER: &str = "CHARACTER";

/// The text "CHARSET", as comparisons test it.
pub(crate) const CHARSET: &str = "CHARSET";

/// The text "COLUMNS", as comparisons test it.
pub(crate) const COLUMNS: &str = "COLUMNS";

/// The text "COMPOUND", as comparisons test it.
pub(crate) const COMPOUND: &str = "COMPOUND";

/// The text "CONTACT", as comparisons test it.
pub(crate) const CONTACT: &str = "CONTACT";

/// The text "DEFINER", as comparisons test it.
pub(crate) const DEFINER: &str = "DEFINER";

/// The text "ENGINE", as comparisons test it.
pub(crate) const ENGINE: &str = "ENGINE";

/// The text "EXCLUDE", as comparisons test it.
pub(crate) const EXCLUDE: &str = "EXCLUDE";

/// The text "FIRST_VALUE", as comparisons test it.
pub(crate) const FIRST_VALUE: &str = "FIRST_VALUE";

/// The text "FUNCTION", as comparisons test it.
pub(crate) const FUNCTION: &str = "FUNCTION";

/// The text "GROUP", as comparisons test it.
pub(crate) const GROUP: &str = "GROUP";

/// The text "OBJECT_AGG", as comparisons test it.
pub(crate) const OBJECT_AGG: &str = "OBJECT_AGG";
/// The text "HASH_AGG", as comparisons test it.
pub(crate) const HASH_AGG: &str = "HASH_AGG";

/// The text "IDENTIFIER", as comparisons test it.
pub(crate) const IDENTIFIER: &str = "IDENTIFIER";

/// The text "INCLUDE", as comparisons test it.
pub(crate) const INCLUDE: &str = "INCLUDE";

/// The text "INTEGER", as comparisons test it.
pub(crate) const INTEGER: &str = "INTEGER";

/// The text "LEAD", as comparisons test it.
pub(crate) const LEAD: &str = "LEAD";

/// The text "LIST", as comparisons test it.
pub(crate) const LIST: &str = "LIST";

/// The text "MAP", as comparisons test it.
pub(crate) const MAP: &str = "MAP";

/// The text "MONTHS_BETWEEN", as comparisons test it.
pub(crate) const MONTHS_BETWEEN: &str = "MONTHS_BETWEEN";

/// The text "MUTEX", as comparisons test it.
pub(crate) const MUTEX: &str = "MUTEX";

/// The text "NATIONAL", as comparisons test it.
pub(crate) const NATIONAL: &str = "NATIONAL";

/// The text "NCHAR", as comparisons test it.
pub(crate) const NCHAR: &str = "NCHAR";

/// The type name of `RETURNS TABLE (...)`, which may have an empty column list.
pub(crate) const TABLE_TYPE: &str = "TABLE";

/// The text "NVARCHAR", as comparisons test it.
pub(crate) const NVARCHAR: &str = "NVARCHAR";

/// The text "d", as comparisons test it.
pub(crate) const ODBC_DATE_ESCAPE: &str = "d";

/// The text "ts", as comparisons test it.
pub(crate) const ODBC_TIMESTAMP_ESCAPE: &str = "ts";

/// The text "t", as comparisons test it.
pub(crate) const ODBC_TIME_ESCAPE: &str = "t";

/// The text "ORDER", as comparisons test it.
pub(crate) const ORDER: &str = "ORDER";

/// The text "%", as comparisons test it.
pub(crate) const PERCENT_SIGN: &str = "%";

/// The text "PLAN", as comparisons test it.
pub(crate) const PLAN: &str = "PLAN";

/// The text "PROCEDURE", as comparisons test it.
pub(crate) const PROCEDURE: &str = "PROCEDURE";

/// The text "s", as comparisons test it.
pub(crate) const PYFORMAT_PARAMETER_SUFFIX: &str = "s";

/// The text "REAL", as comparisons test it.
pub(crate) const REAL: &str = "REAL";

/// The text "REPRODUCTION", as comparisons test it.
pub(crate) const REPRODUCTION: &str = "REPRODUCTION";

/// The text "RETURNING", as comparisons test it.
pub(crate) const RETURNING: &str = "RETURNING";

/// The text "ROW", as comparisons test it.
pub(crate) const ROW: &str = "ROW";

/// The text "SCHEDULE", as comparisons test it.
pub(crate) const SCHEDULE: &str = "SCHEDULE";

/// The text "SQL", as comparisons test it.
pub(crate) const SQL: &str = "SQL";

/// The text "STATUS", as comparisons test it.
pub(crate) const STATUS: &str = "STATUS";

/// The text "STRUCT", as comparisons test it.
pub(crate) const STRUCT: &str = "STRUCT";

/// The text "TAG", as comparisons test it.
pub(crate) const TAG: &str = "TAG";

/// The text "TIME_TO_TIME_STR", as comparisons test it.
pub(crate) const TIME_TO_TIME_STR: &str = "TIME_TO_TIME_STR";

/// The text "TRY_CONVERT", as comparisons test it.
pub(crate) const TRY_CONVERT: &str = "TRY_CONVERT";

/// The text "TRY_TO_NUMBER", as comparisons test it.
pub(crate) const TRY_TO_NUMBER: &str = "TRY_TO_NUMBER";

/// The text "UNSIGNED", as comparisons test it.
pub(crate) const UNSIGNED: &str = "UNSIGNED";

/// The text "USER", as comparisons test it.
pub(crate) const USER: &str = "USER";

/// The text "VAR", as comparisons test it.
pub(crate) const VAR: &str = "VAR";

/// The text "VARIABLE", as comparisons test it.
pub(crate) const VARIABLE: &str = "VARIABLE";

/// The text "ZEROFILL", as comparisons test it.
pub(crate) const ZEROFILL: &str = "ZEROFILL";

/// The text "ACCOUNT", as comparisons test it.
pub(crate) const ACCOUNT: &str = "ACCOUNT";

/// The text "DROP ROW ACCESS POLICY", as comparisons test it.
pub(crate) const DROP_ROW_ACCESS_POLICY: &str = "DROP ROW ACCESS POLICY";

/// The text "ON ACCOUNT", as comparisons test it.
pub(crate) const ON_ACCOUNT: &str = "ON ACCOUNT";

/// The text ")", as comparisons test it.
pub(crate) const RIGHT_PAREN: &str = ")";

/// The text "SET", as comparisons test it.
pub(crate) const SET: &str = "SET";

/// The text "SET MASKING POLICY", as comparisons test it.
pub(crate) const SET_MASKING_POLICY: &str = "SET MASKING POLICY";

/// The text "SET TAG", as comparisons test it.
pub(crate) const SET_TAG: &str = "SET TAG";

/// The text "UNSET COMMENT", as comparisons test it.
pub(crate) const UNSET_COMMENT: &str = "UNSET COMMENT";

/// The argument count of a two-argument call.
pub(crate) const BINARY_ARITY: usize = 2;

/// The argument count of a three-argument call.
pub(crate) const TERNARY_ARITY: usize = 3;

/// The parts of a two-part qualified name, `schema.name`.
pub(crate) const QUALIFIED_NAME_PARTS: usize = 2;

/// How far back the token before the previous one is.
pub(crate) const SECOND_PREVIOUS_OFFSET: usize = 2;

/// The start of a T-SQL `OPTION(...)` query hint as the parser rebuilds it.
pub(crate) const OPTION_CLAUSE_OPEN: &str = "OPTION(";

/// The length of the shortest quoted text: its two quotes.
pub(crate) const QUOTED_TEXT_MIN_LEN: usize = 2;

/// PostgreSQL's streaming-replication commands, tokenized as commands as polyglot does.
pub(crate) const POSTGRES_REPLICATION_COMMANDS: [&str; 7] = [
    "BASE_BACKUP",
    "CREATE_REPLICATION_SLOT",
    "DROP_REPLICATION_SLOT",
    "IDENTIFY_SYSTEM",
    "READ_REPLICATION_SLOT",
    "START_REPLICATION",
    "TIMELINE_HISTORY",
];

/// PostgreSQL's prefix for an argument that spreads an array over a variadic parameter.
pub(crate) const VARIADIC_KEYWORD: &str = "VARIADIC";

/// The EXPLAIN option that runs the statement.
pub(crate) const EXPLAIN_ANALYZE: &str = "ANALYZE";

/// The EXPLAIN option values that turn an option on.
pub(crate) const EXPLAIN_TRUE_VALUES: [&str; 3] = ["TRUE", "ON", "1"];

/// The tokens after which SQLite's rewritten `NOT (x IN (...))` keeps parentheses.
pub(crate) const SQLITE_RANGE_FOLLOWERS: [sqlquack_syntax::tokens::models::TokenType; 9] = [
    sqlquack_syntax::tokens::models::TokenType::Not,
    sqlquack_syntax::tokens::models::TokenType::In,
    sqlquack_syntax::tokens::models::TokenType::Is,
    sqlquack_syntax::tokens::models::TokenType::Between,
    sqlquack_syntax::tokens::models::TokenType::Like,
    sqlquack_syntax::tokens::models::TokenType::ILike,
    sqlquack_syntax::tokens::models::TokenType::RLike,
    sqlquack_syntax::tokens::models::TokenType::Glob,
    sqlquack_syntax::tokens::models::TokenType::Match,
];

/// Snowflake types written before a string as a typed literal, beyond the shared ones.
pub(crate) const SNOWFLAKE_LITERAL_TYPES: &[&str] = &["IPADDRESS"];

/// The names Snowflake calls in FROM without TABLE( ); any other `name ( ... )` there is refused (errno 1003).
pub(crate) const SNOWFLAKE_FROM_CALLS: &[&str] = &[
    "DIRECTORY",
    "DYNAMIC_TABLE_REFRESH_BOUNDARY",
    "ERROR_TABLE",
    "IDENTIFIER",
    "SEMANTIC_VIEW",
    "TABLE",
];

/// The properties a column-form Snowflake external table needs; the first is its stage location.
pub(crate) const SNOWFLAKE_EXTERNAL_TABLE_REQUIRED: &[&str] = &["LOCATION", "FILE_FORMAT"];

/// The `<key> = <value>` properties an iceberg table takes besides a table's.
pub(crate) const SNOWFLAKE_ICEBERG_TABLE_KEYS: &[&str] = &[
    "ALLOW_V3_EXPRESSION_DEFAULTS",
    "AUTO_REFRESH",
    "BASE_LOCATION",
    "CATALOG",
    "CATALOG_NAMESPACE",
    "CATALOG_SYNC",
    "CATALOG_TABLE_NAME",
    "ENABLE_DATA_COMPACTION",
    "ENABLE_ICEBERG_MERGE_ON_READ",
    "ENABLE_SCHEMA_INFERENCE",
    "EXTERNAL_VOLUME",
    "ICEBERG_MERGE_ON_READ_BEHAVIOR",
    "ICEBERG_VERSION",
    "MATCH_BY_COLUMN_NAME",
    "METADATA_FILE_PATH",
    "PATH_LAYOUT",
    "PATTERN",
    "REFRESH_ON_CREATE",
    "REPLACE_INVALID_CHARACTERS",
    "STORAGE_SERIALIZATION_POLICY",
    "TARGET_FILE_SIZE",
];

/// The `<key> = <value>` properties a dynamic table takes besides a table's.
pub(crate) const SNOWFLAKE_DYNAMIC_TABLE_KEYS: &[&str] = &[
    "INITIALIZATION_WAREHOUSE",
    "INITIALIZE",
    "REFRESH_MODE",
    "SCHEDULER",
];

/// The properties every dynamic table needs.
pub(crate) const SNOWFLAKE_DYNAMIC_TABLE_REQUIRED: &[&str] = &["TARGET_LAG", "WAREHOUSE"];

/// The `<key> = <value>` properties a Snowflake CREATE TABLE takes.
pub(crate) const SNOWFLAKE_TABLE_PROPERTY_KEYS: &[&str] = &[
    "CHANGE_TRACKING",
    "COMMENT",
    "DATA_RETENTION_TIME_IN_DAYS",
    "DEFAULT_DDL_COLLATION",
    "ENABLE_SCHEMA_EVOLUTION",
    "ERROR_LOGGING",
    "ICEBERG_DEFAULT_DDL_COLLATION",
    "MAX_DATA_EXTENSION_TIME_IN_DAYS",
    "ROW_TIMESTAMP",
    "TARGET_LAG",
    "WAREHOUSE",
];

/// The fields every semantic view verified query needs.
pub(crate) const SNOWFLAKE_VERIFIED_QUERY_FIELDS: &[&str] = &["QUESTION", "SQL"];

/// The clause words of a SEMANTIC_VIEW call, which cannot be an alias there.
pub(crate) const SNOWFLAKE_SEMANTIC_VIEW_CLAUSES: &[&str] =
    &["DIMENSIONS", "FACTS", "METRICS", "WHERE"];

/// Words that end a MEASURES item in Snowflake, so they cannot be its alias.
pub(crate) const SNOWFLAKE_MEASURES_ENDS: &[&str] = &["AFTER", "ALL", "DEFINE", "ONE", "PATTERN"];

/// FROM calls whose one argument is an object name: none, or a string, is refused (errno 1003).
pub(crate) const SNOWFLAKE_FROM_CALLS_OF_A_NAME: &[&str] =
    &["DYNAMIC_TABLE_REFRESH_BOUNDARY", "ERROR_TABLE"];

/// Tokens after which a Snowflake PRIOR has no operand, so the word is a name.
pub(crate) const SNOWFLAKE_PRIOR_ENDS: &[sqlquack_syntax::tokens::models::TokenType] = &[
    sqlquack_syntax::tokens::models::TokenType::Eq,
    sqlquack_syntax::tokens::models::TokenType::Neq,
    sqlquack_syntax::tokens::models::TokenType::Lt,
    sqlquack_syntax::tokens::models::TokenType::Lte,
    sqlquack_syntax::tokens::models::TokenType::Gt,
    sqlquack_syntax::tokens::models::TokenType::Gte,
    sqlquack_syntax::tokens::models::TokenType::And,
    sqlquack_syntax::tokens::models::TokenType::Or,
    sqlquack_syntax::tokens::models::TokenType::RParen,
    sqlquack_syntax::tokens::models::TokenType::Comma,
    sqlquack_syntax::tokens::models::TokenType::Semicolon,
    sqlquack_syntax::tokens::models::TokenType::Start,
    sqlquack_syntax::tokens::models::TokenType::Order,
];

/// Reserved words Snowflake is recorded refusing as an unquoted later part of a dotted name.
pub(crate) const SNOWFLAKE_REFUSED_NAME_PARTS: &[&str] = &["SELECT"];

/// Longer than any Snowflake reserved keyword, so a longer word is none.
pub(crate) const SNOWFLAKE_RESERVED_LONGEST: usize = 32;

/// Snowflake's fully reserved keywords; its position-limited keywords are not listed.
pub(crate) const SNOWFLAKE_RESERVED_KEYWORDS: &[&str] = &[
    "ALL",
    "ALTER",
    "AND",
    "ANY",
    "AS",
    "BETWEEN",
    "BY",
    "CHECK",
    "COLUMN",
    "CONNECT",
    "CREATE",
    "CURRENT",
    "DELETE",
    "DISTINCT",
    "DROP",
    "ELSE",
    "EXISTS",
    "FOLLOWING",
    "FOR",
    "FROM",
    "GRANT",
    "GROUP",
    "HAVING",
    "ILIKE",
    "IN",
    "INCREMENT",
    "INSERT",
    "INTERSECT",
    "INTO",
    "IS",
    "LIKE",
    "MINUS",
    "NOT",
    "NULL",
    "OF",
    "ON",
    "OR",
    "ORDER",
    "QUALIFY",
    "REGEXP",
    "REVOKE",
    "RLIKE",
    "ROW",
    "ROWS",
    "SAMPLE",
    "SELECT",
    "SET",
    "SOME",
    "START",
    "TABLE",
    "TABLESAMPLE",
    "THEN",
    "TO",
    "TRIGGER",
    "UNION",
    "UNIQUE",
    "UPDATE",
    "VALUES",
    "WHENEVER",
    "WHERE",
    "WINDOW",
    "WITH",
];

/// The words that start a Snowflake ALTER statement's object kind; any other word is no ALTER.
pub(crate) const SNOWFLAKE_ALTER_KINDS: &[&str] = &[
    "ACCOUNT",
    "AGENT",
    "AGGREGATION",
    "ALERT",
    "API",
    "APPLICATION",
    "ARTIFACT",
    "AUTHENTICATION",
    "BACKUP",
    "CATALOG",
    "COMPUTE",
    "CONNECTION",
    "CONTACT",
    "CORTEX",
    "DATA",
    "DATABASE",
    "DATASET",
    "DBT",
    "DCM",
    "DEPLOYMENT",
    "DYNAMIC",
    "EVENT",
    "EXPERIMENT",
    "EXTERNAL",
    "FAILOVER",
    "FEATURE",
    "FILE",
    "FUNCTION",
    "GATEWAY",
    "GIT",
    "ICEBERG",
    "INDEX",
    "INGRESS",
    "INTEGRATION",
    "JOIN",
    "LISTING",
    "MAINTENANCE",
    "MASKING",
    "MATERIALIZED",
    "MODEL",
    "MULTI",
    "NETWORK",
    "NOTEBOOK",
    "NOTIFICATION",
    "ONLINE",
    "OPENFLOW",
    "ORGANIZATION",
    "PACKAGES",
    "PASSWORD",
    "PIPE",
    "POSTGRES",
    "PRIVACY",
    "PROCEDURE",
    "PROJECTION",
    "REPLICATION",
    "RESOURCE",
    "RESTRICTED",
    "ROLE",
    "ROW",
    "RSS",
    "SCHEMA",
    "SECRET",
    "SECURITY",
    "SEMANTIC",
    "SEQUENCE",
    "SERVICE",
    "SESSION",
    "SHARE",
    "SNAPSHOT",
    "SNOWFLAKE",
    "STAGE",
    "STORAGE",
    "STREAM",
    "STREAMLIT",
    "TABLE",
    "TAG",
    "TASK",
    "TYPE",
    "USER",
    "VARIABLE",
    "VIEW",
    "VOLUME",
    "WAREHOUSE",
];

/// The kind of `EXECUTE DCM PROJECT`, whose actions differ from a task's.
pub(crate) const EXECUTE_DCM_PROJECT: &str = "DCM PROJECT";

/// The words that start an ALTER TASK action, so `ALTER TASK GRAPH <action>` alters the task named GRAPH.
pub(crate) const TASK_ACTIONS: &[&str] = &[
    "ADD", "MODIFY", "REMOVE", "RESUME", "SET", "SUSPEND", "UNSET",
];

/// Snowflake DROP object kinds, each listed before any kind it prefixes.
pub(crate) const SNOWFLAKE_DROP_KINDS: &[&[&str]] = &[
    &["AGENT"],
    &["AGGREGATION", "POLICY"],
    &["ALERT"],
    &["API", "INTEGRATION"],
    &["APPLICATION", "PACKAGE"],
    &["APPLICATION", "SERVICE"],
    &["APPLICATION", "ROLE"],
    &["APPLICATION"],
    &["ARTIFACT", "REPOSITORY"],
    &["AUTHENTICATION", "POLICY"],
    &["BACKUP", "POLICY"],
    &["BACKUP", "SET"],
    &["CATALOG", "INTEGRATION"],
    &["COMPUTE", "POOL"],
    &["CONNECTION"],
    &["CONTACT"],
    &["CORTEX", "SEARCH", "SERVICE"],
    &["DATA", "MOVEMENT", "POLICY"],
    &["DATA", "MOVEMENT", "RULE"],
    &["DBT", "PROJECT"],
    &["DCM", "PROJECT"],
    &["DYNAMIC", "TABLE"],
    &["EVENT", "ROUTING", "TABLE"],
    &["EXPERIMENT"],
    &["EXTERNAL", "ACCESS", "INTEGRATION"],
    &["EXTERNAL", "AGENT"],
    &["EXTERNAL", "CONSUMER"],
    &["EXTERNAL", "TABLE"],
    &["EXTERNAL", "VOLUME"],
    &["FAILOVER", "GROUP"],
    &["FEATURE", "POLICY"],
    &["FILE", "FORMAT"],
    &["GATEWAY"],
    &["GIT", "REPOSITORY"],
    &["IMAGE", "REPOSITORY"],
    &["INGRESS", "POLICY"],
    &["INTEGRATION"],
    &["JOIN", "POLICY"],
    &["LISTING"],
    &["MAINTENANCE", "POLICY"],
    &["MASKING", "POLICY"],
    &["MCP", "SERVER"],
    &["MODEL", "MONITOR"],
    &["MODEL"],
    &["MULTI", "PARTY", "APPROVAL", "POLICY"],
    &["NETWORK", "POLICY"],
    &["NETWORK", "RULE"],
    &["NOTEBOOK"],
    &["NOTIFICATION", "INTEGRATION"],
    &["ONLINE", "FEATURE", "TABLE"],
    &["OPENFLOW", "CONNECTOR"],
    &["OPENFLOW", "DEPLOYMENT"],
    &["OPENFLOW", "RUNTIME"],
    &["ORGANIZATION", "USER", "GROUP"],
    &["ORGANIZATION", "USER"],
    &["ORGANIZATION", "PROFILE"],
    &["ORGANIZATION"],
    &["PACKAGES", "POLICY"],
    &["PASSWORD", "POLICY"],
    &["PIPE"],
    &["POSTGRES", "INSTANCE"],
    &["PRIVACY", "POLICY"],
    &["PROFILE"],
    &["PROJECTION", "POLICY"],
    &["REPLICATION", "GROUP"],
    &["RESOURCE", "MONITOR"],
    &["RESTRICTED", "SESSION", "SCOPE"],
    &["ROLE"],
    &["ROW", "ACCESS", "POLICY"],
    &["RSS"],
    &["SECRET"],
    &["SECURITY", "INTEGRATION"],
    &["SEMANTIC", "VIEW"],
    &["SERVICE"],
    &["SESSION", "POLICY"],
    &["SHARE"],
    &["SNAPSHOT", "POLICY"],
    &["SNAPSHOT", "SET"],
    &["SNAPSHOT"],
    &["STAGE"],
    &["STORAGE", "INTEGRATION"],
    &["STORAGE", "LIFECYCLE", "POLICY"],
    &["STREAMLIT"],
    &["STREAM"],
    &["TAG"],
    &["TASK", "GRAPH"],
    &["TASK"],
    &["USER"],
    &["VARIABLE"],
    &["WAREHOUSE"],
];

/// Snowflake DESCRIBE kinds the shared DESCRIBE does not read, each before any kind it prefixes.
pub(crate) const SNOWFLAKE_DESCRIBE_KINDS: &[&[&str]] = &[
    &["AGENT"],
    &["ALERT"],
    &["APPLICATION", "PACKAGE"],
    &["APPLICATION", "SERVICE"],
    &["APPLICATION"],
    &["ARTIFACT", "REPOSITORY"],
    &["AVAILABLE", "LISTING"],
    &["AVAILABLE", "ORGANIZATION", "PROFILE"],
    &["BACKUP", "POLICY"],
    &["BACKUP", "SET"],
    &["COMMITTED", "TASK"],
    &["COMPUTE", "POOL"],
    &["CONFIGURATION"],
    &["CORTEX", "SEARCH", "SERVICE"],
    &["DATABASE"],
    &["DATA", "MOVEMENT", "POLICY"],
    &["DATA", "MOVEMENT", "RULE"],
    &["DBT", "PROJECT"],
    &["DCM", "PROJECT"],
    &["DYNAMIC", "TABLE"],
    &["EVENT", "TABLE"],
    &["EXPERIMENT"],
    &["EXTERNAL", "ACCESS", "INTEGRATION"],
    &["EXTERNAL", "AGENT"],
    &["EXTERNAL", "CONSUMER"],
    &["EXTERNAL", "TABLE"],
    &["EXTERNAL", "VOLUME"],
    &["FILE", "FORMAT"],
    &["GATEWAY"],
    &["GIT", "REPOSITORY"],
    &["ICEBERG", "TABLE"],
    &["LISTING"],
    &["MATERIALIZED", "VIEW"],
    &["MCP", "SERVER"],
    &["MODEL", "MONITOR"],
    &["NOTEBOOK"],
    &["ONLINE", "FEATURE", "TABLE"],
    &["OPENFLOW", "CONNECTOR"],
    &["OPENFLOW", "DATA", "PLANE", "INTEGRATION"],
    &["OPENFLOW", "DEPLOYMENT"],
    &["OPENFLOW", "RUNTIME"],
    &["ORGANIZATION", "PROFILE"],
    &["ORGANIZATION"],
    &["PROFILE"],
    &["PIPE"],
    &["POSTGRES", "INSTANCE"],
    &["RESTRICTED", "SESSION", "SCOPE"],
    &["RESULT"],
    &["RSS"],
    &["SCHEMA"],
    &["SEARCH", "OPTIMIZATION", "ON"],
    &["SECRET"],
    &["SEMANTIC", "VIEW"],
    &["SEQUENCE"],
    &["SERVICE"],
    &["SHARE"],
    &["SNAPSHOT", "POLICY"],
    &["SNAPSHOT", "SET"],
    &["SNAPSHOT"],
    &["SPECIFICATION"],
    &["STAGE"],
    &["STREAMLIT"],
    &["STREAM"],
    &["TASK", "GRAPH"],
    &["TASK"],
    &["TRANSACTION"],
    &["TYPE"],
    &["USER"],
    &["VARIABLE"],
    &["WAREHOUSE"],
    &["AGGREGATION", "POLICY"],
    &["AUTHENTICATION", "POLICY"],
    &["FEATURE", "POLICY"],
    &["INGRESS", "POLICY"],
    &["JOIN", "POLICY"],
    &["MAINTENANCE", "POLICY"],
    &["MASKING", "POLICY"],
    &["MULTI", "PARTY", "APPROVAL", "POLICY"],
    &["NETWORK", "POLICY"],
    &["NETWORK", "RULE"],
    &["PACKAGES", "POLICY"],
    &["PASSWORD", "POLICY"],
    &["PRIVACY", "POLICY"],
    &["PROJECTION", "POLICY"],
    &["ROW", "ACCESS", "POLICY"],
    &["SESSION", "POLICY"],
    &["STORAGE", "LIFECYCLE", "POLICY"],
    &["API", "INTEGRATION"],
    &["CATALOG", "INTEGRATION"],
    &["NOTIFICATION", "INTEGRATION"],
    &["SECURITY", "INTEGRATION"],
    &["STORAGE", "INTEGRATION"],
    &["INTEGRATION"],
];

/// The kinds the shared DESCRIBE reads for Snowflake.
pub(crate) const SHARED_DESCRIBE_KINDS: &[&str] = &["TABLE", "VIEW", "FUNCTION", "PROCEDURE"];

/// Snowflake SHOW kinds, each before any kind it prefixes.
pub(crate) const SNOWFLAKE_SHOW_KINDS: &[&[&str]] = &[
    &[
        "EVENT",
        "ROUTING",
        "TABLE",
        "ON",
        "ORGANIZATION",
        "FOR",
        "ALL",
        "APPLICATION",
        "LISTINGS",
    ],
    &["USER", "WORKLOAD", "IDENTITY", "AUTHENTICATION", "METHODS"],
    &["USER", "PROGRAMMATIC", "ACCESS", "TOKENS"],
    &["USER", "KEY", "PAIRS"],
    &["USER", "PATS"],
    &["FEATURE", "POLICIES"],
    &["MAINTENANCE", "POLICIES"],
    &["COMPUTE", "POOL", "INSTANCE", "FAMILIES"],
    &["DELEGATED", "AUTHORIZATIONS", "BY", "USER"],
    &["MULTI", "PARTY", "APPROVAL", "POLICIES"],
    &["OPENFLOW", "DATA", "PLANE", "INTEGRATIONS"],
    &["USER", "FUNCTIONS"],
    &["USER", "PROCEDURES"],
    &["AVAILABLE", "ORGANIZATION", "PROFILES"],
    &["CORTEX", "BASE", "MODELS"],
    &["DATA", "METRIC", "FUNCTIONS"],
    &["DATA", "MOVEMENT", "POLICIES"],
    &["DATA", "MOVEMENT", "RULES"],
    &["EVENT", "ROUTING", "TABLES"],
    &["EXTERNAL", "ACCESS", "INTEGRATIONS"],
    &["ONLINE", "FEATURE", "TABLES"],
    &["OPENFLOW", "CONNECTOR", "DEFINITIONS"],
    &["ORGANIZATION", "USER", "GROUPS"],
    &["RESTRICTED", "SESSION", "SCOPES"],
    &["ROW", "ACCESS", "POLICIES"],
    &["STORAGE", "LIFECYCLE", "POLICIES"],
    &["TELEMETRY", "EVENT", "DEFINITIONS"],
    &["AGGREGATION", "POLICIES"],
    &["API", "INTEGRATIONS"],
    &["APPLICATION", "PACKAGES"],
    &["APPLICATION", "ROLES"],
    &["APPLICATION", "SERVICES"],
    &["APPROVED", "SPECIFICATIONS"],
    &["ARTIFACT", "REPOSITORIES"],
    &["AUTHENTICATION", "POLICIES"],
    &["AVAILABLE", "OFFERS"],
    &["BACKUP", "POLICIES"],
    &["BACKUP", "SETS"],
    &["CATALOG", "INTEGRATIONS"],
    &["CODE", "BUNDLES"],
    &["COMMITTED", "TASKS"],
    &["COMPUTE", "POOLS"],
    &["DATABASE", "ROLES"],
    &["DBT", "PROJECTS"],
    &["DCM", "PROJECTS"],
    &["DECLINED", "SPECIFICATIONS"],
    &["DELEGATED", "AUTHORIZATIONS"],
    &["DYNAMIC", "TABLES"],
    &["EVENT", "TABLES"],
    &["EXTERNAL", "AGENTS"],
    &["EXTERNAL", "CONSUMERS"],
    &["EXTERNAL", "FUNCTIONS"],
    &["EXTERNAL", "TABLES"],
    &["EXTERNAL", "VOLUMES"],
    &["FAILOVER", "GROUPS"],
    &["FILE", "FORMATS"],
    &["GIT", "BRANCHES"],
    &["GIT", "REPOSITORIES"],
    &["GIT", "TAGS"],
    &["GLOBAL", "ACCOUNTS"],
    &["HYBRID", "TABLES"],
    &["ICEBERG", "TABLES"],
    &["IMAGE", "REPOSITORIES"],
    &["INGRESS", "POLICIES"],
    &["INTERACTIVE", "TABLES"],
    &["JOIN", "POLICIES"],
    &["MANAGED", "ACCOUNTS"],
    &["MASKING", "POLICIES"],
    &["MATERIALIZED", "VIEWS"],
    &["MCP", "SERVERS"],
    &["MFA", "METHODS"],
    &["MODEL", "MONITORS"],
    &["NETWORK", "POLICIES"],
    &["NETWORK", "RULES"],
    &["NOTEBOOK", "PROJECTS"],
    &["NOTIFICATION", "INTEGRATIONS"],
    &["OPENFLOW", "CONNECTORS"],
    &["OPENFLOW", "DEPLOYMENTS"],
    &["OPENFLOW", "RUNTIMES"],
    &["ORGANIZATION", "ACCOUNTS"],
    &["ORGANIZATION", "CONTRACTS"],
    &["ORGANIZATION", "PROFILES"],
    &["ORGANIZATION", "USERS"],
    &["PACKAGES", "POLICIES"],
    &["PASSWORD", "POLICIES"],
    &["PENDING", "SPECIFICATIONS"],
    &["POSTGRES", "INSTANCES"],
    &["PRICING", "PLANS"],
    &["PRIVACY", "POLICIES"],
    &["PROJECTION", "POLICIES"],
    &["RELEASE", "CHANNELS"],
    &["RELEASE", "DIRECTIVES"],
    &["REPLICATION", "ACCOUNTS"],
    &["REPLICATION", "DATABASES"],
    &["REPLICATION", "GROUPS"],
    &["RESOURCE", "MONITORS"],
    &["RUN", "METRICS"],
    &["RUN", "PARAMETERS"],
    &["SCHEMA", "VARIABLES"],
    &["SECURITY", "INTEGRATIONS"],
    &["SEMANTIC", "DIMENSIONS"],
    &["SEMANTIC", "FACTS"],
    &["SEMANTIC", "METRICS"],
    &["SEMANTIC", "VIEWS"],
    &["SERVICE", "CONTAINERS"],
    &["SERVICE", "INSTANCES"],
    &["SERVICE", "VOLUMES"],
    &["SESSION", "POLICIES"],
    &["SHARED", "CONTENT"],
    &["SNAPSHOT", "POLICIES"],
    &["SNAPSHOT", "SETS"],
    &["STORAGE", "INTEGRATIONS"],
    &["TASK", "GRAPHS"],
    &["ACCOUNTS"],
    &["AGENTS"],
    &["ALERTS"],
    &["APPLICATIONS"],
    &["BACKUPS"],
    &["CHANNELS"],
    &["CLASSES"],
    &["COLUMNS"],
    &["CONFIGURATIONS"],
    &["CONNECTIONS"],
    &["CONTACTS"],
    &["DATABASES"],
    &["DATASETS"],
    &["DEPLOYMENTS"],
    &["ENDPOINTS"],
    &["ENTITIES"],
    &["EXPERIMENTS"],
    &["FILES"],
    &["FUNCTIONS"],
    &["GATEWAYS"],
    &["IMAGES"],
    &["INDEXES"],
    &["INTEGRATIONS"],
    &["LISTINGS"],
    &["LOCKS"],
    &["MODELS"],
    &["NODES"],
    &["NOTEBOOKS"],
    &["OBJECTS"],
    &["OFFERS"],
    &["PACKAGES"],
    &["PIPES"],
    &["PRIVILEGES"],
    &["PROCEDURES"],
    &["REFERENCES"],
    &["REGIONS"],
    &["ROLES"],
    &["RSS"],
    &["RUNS"],
    &["SCHEMAS"],
    &["SECRETS"],
    &["SEQUENCES"],
    &["SHARES"],
    &["SNAPSHOTS"],
    &["SPECIFICATIONS"],
    &["STAGES"],
    &["STREAMLITS"],
    &["STREAMS"],
    &["TABLES"],
    &["TAGS"],
    &["TASKS"],
    &["TRANSACTIONS"],
    &["TYPES"],
    &["USERS"],
    &["VARIABLES"],
    &["VERSIONS"],
    &["VIEWS"],
    &["WAREHOUSES"],
    &["WORKSPACES"],
];

/// The one-word object kinds UNDROP takes besides TABLE, SCHEMA and DATABASE.
pub(crate) const SNOWFLAKE_UNDROP_KINDS: &[&str] = &[
    "ACCOUNT",
    "NOTEBOOK",
    "SNAPSHOT",
    "STAGE",
    "STREAMLIT",
    "TAG",
    "TYPE",
    "VARIABLE",
];

/// SHOW kinds the shared SHOW still reads for Snowflake, by their leading words.
pub(crate) const SHARED_SHOW_KINDS: &[&[&str]] = &[
    &["CORTEX", "SEARCH", "SERVICES"],
    &["EXPORTED", "KEYS"],
    &["IMPORTED", "KEYS"],
    &["PRIMARY", "KEYS"],
    &["UNIQUE", "KEYS"],
    &["AVAILABLE", "LISTINGS"],
    &["JOB"],
    &["PARAMETERS"],
    &["SERVICES"],
];

/// The containers a Snowflake SHOW ... IN names, each before any container it prefixes.
pub(crate) const SNOWFLAKE_SHOW_CONTAINERS: &[&[&str]] = &[
    &["EVENT", "ROUTING", "TABLE"],
    &["ORGANIZATION", "USER", "GROUP"],
    &["APPLICATION", "PACKAGE"],
    &["ARTIFACT", "REPOSITORY"],
    &["BACKUP", "SET"],
    &["COMPUTE", "POOL"],
    &["DBT", "PROJECT"],
    &["DCM", "PROJECT"],
    &["FAILOVER", "GROUP"],
    &["GIT", "REPOSITORY"],
    &["IMAGE", "REPOSITORY"],
    &["OPENFLOW", "CONNECTOR"],
    &["ORGANIZATION", "PROFILE"],
    &["ORGANIZATION"],
    &["REPLICATION", "GROUP"],
    &["SNAPSHOT", "SET"],
    &["ACCOUNT"],
    &["APPLICATION"],
    &["CLASS"],
    &["DATABASE"],
    &["DATASET"],
    &["EXPERIMENT"],
    &["GRAPH"],
    &["LISTING"],
    &["MODEL"],
    &["SCHEMA"],
    &["SERVICE"],
    &["TABLE"],
    &["VIEW"],
];

/// The containers any Snowflake SHOW ... IN may name; "" is a bare name.
pub(crate) const SNOWFLAKE_SHOW_GENERAL_CONTAINERS: &[&str] = &[
    "",
    "ACCOUNT",
    "APPLICATION",
    "APPLICATION PACKAGE",
    "CLASS",
    "DATABASE",
    "SCHEMA",
    "TABLE",
    "VIEW",
];

/// Further containers some Snowflake SHOW kinds may name.
pub(crate) const SNOWFLAKE_SHOW_EXTRA_CONTAINERS: &[(&str, &[&str])] = &[
    ("COMMITTED TASKS", &["GRAPH"]),
    ("DATABASES", &["FAILOVER GROUP", "REPLICATION GROUP"]),
    ("FUNCTIONS", &["MODEL"]),
    ("LISTINGS", &["FAILOVER GROUP"]),
    ("ORGANIZATION USERS", &["ORGANIZATION USER GROUP"]),
    ("ROLES", &["SERVICE"]),
    ("SHARES", &["FAILOVER GROUP", "REPLICATION GROUP"]),
    ("SNAPSHOTS", &["SNAPSHOT SET"]),
];

/// Snowflake SHOW kinds that need IN and one of these containers; "" is a bare name.
pub(crate) const SNOWFLAKE_SHOW_REQUIRED_CONTAINERS: &[(&str, &[&str])] = &[
    ("APPLICATION ROLES", &["APPLICATION", ""]),
    ("AVAILABLE OFFERS", &["LISTING"]),
    ("BACKUPS", &["BACKUP SET"]),
    ("DATABASE ROLES", &["DATABASE", ""]),
    ("DEPLOYMENTS", &["DCM PROJECT"]),
    ("ENDPOINTS", &["SERVICE"]),
    ("ENTITIES", &["DCM PROJECT"]),
    ("FILES", &["ARTIFACT REPOSITORY"]),
    ("GIT BRANCHES", &["GIT REPOSITORY", ""]),
    ("GIT TAGS", &["GIT REPOSITORY", ""]),
    ("IMAGES", &["IMAGE REPOSITORY"]),
    ("NODES", &["COMPUTE POOL"]),
    ("OFFERS", &["LISTING"]),
    ("PACKAGES", &["ARTIFACT REPOSITORY"]),
    ("PRICING PLANS", &["LISTING"]),
    ("PRIVILEGES", &["APPLICATION"]),
    ("REFERENCES", &["APPLICATION"]),
    ("RELEASE CHANNELS", &["APPLICATION PACKAGE", "LISTING"]),
    ("RELEASE DIRECTIVES", &["APPLICATION PACKAGE"]),
    ("RUN METRICS", &["EXPERIMENT"]),
    ("RUN PARAMETERS", &["EXPERIMENT"]),
    ("RUNS", &["EXPERIMENT"]),
    ("SCHEMA VARIABLES", &["SCHEMA"]),
    ("SERVICE CONTAINERS", &["SERVICE"]),
    ("SERVICE INSTANCES", &["SERVICE"]),
    ("SERVICE VOLUMES", &["SERVICE"]),
    ("SHARED CONTENT", &["APPLICATION PACKAGE"]),
    ("TELEMETRY EVENT DEFINITIONS", &["APPLICATION"]),
    (
        "VERSIONS",
        &[
            "APPLICATION PACKAGE",
            "ARTIFACT REPOSITORY",
            "DATASET",
            "DBT PROJECT",
            "LISTING",
            "MODEL",
            "OPENFLOW CONNECTOR",
            "ORGANIZATION PROFILE",
            "ORGANIZATION",
        ],
    ),
];

/// The interval field whose precision after TO counts fractional digits.
pub(crate) const SNOWFLAKE_INTERVAL_SECOND: &str = "SECOND";

/// Snowflake's recorded leading-field interval precisions, as in `INTERVAL YEAR ( 9 )`.
pub(crate) const SNOWFLAKE_INTERVAL_LEADING: std::ops::RangeInclusive<u8> = 1..=9;

/// Snowflake's recorded fractional-second precisions after TO, as in `DAY TO SECOND ( 0 )`.
pub(crate) const SNOWFLAKE_INTERVAL_FRACTION: std::ops::RangeInclusive<u8> = 0..=9;

/// The backtick that quotes identifiers in other dialects but not in Snowflake.
pub(crate) const BACKTICK: &str = "`";

/// Snowflake function and procedure properties whose value is a string.
pub(crate) const SNOWFLAKE_ROUTINE_TEXT_PROPERTIES: &[&str] = &[
    "COMMENT",
    "HANDLER",
    "LOG_LEVEL",
    "METRIC_LEVEL",
    "RUNTIME_VERSION",
    "TARGET_PATH",
    "TRACE_LEVEL",
];

/// Snowflake function and procedure properties whose value is a name.
pub(crate) const SNOWFLAKE_ROUTINE_NAME_PROPERTIES: &[&str] = &[
    "API_INTEGRATION",
    "ARTIFACT_REPOSITORY",
    "COMPRESSION",
    "ENDPOINT",
    "ON_BATCH_FAILURE",
    "REQUEST_TRANSLATOR",
    "RESPONSE_TRANSLATOR",
    "SERVICE",
];

/// Snowflake function and procedure properties whose value is a number.
pub(crate) const SNOWFLAKE_ROUTINE_NUMBER_PROPERTIES: &[&str] =
    &["BATCH_TIMEOUT_SECS", "MAX_BATCH_RETRIES", "MAX_BATCH_ROWS"];

/// Snowflake function and procedure properties whose value is a parenthesized string list.
pub(crate) const SNOWFLAKE_ROUTINE_TEXT_LIST_PROPERTIES: &[&str] =
    &["ARTIFACT_REPOSITORY_PACKAGES", "IMPORTS", "PACKAGES"];

/// Snowflake function and procedure properties whose value is a parenthesized name list.
pub(crate) const SNOWFLAKE_ROUTINE_NAME_LIST_PROPERTIES: &[&str] =
    &["CONTEXT_HEADERS", "COPY", "EXTERNAL_ACCESS_INTEGRATIONS"];

/// Snowflake function and procedure properties whose names, in a single value, a list or a map, may also be strings.
pub(crate) const SNOWFLAKE_ROUTINE_TEXT_NAME_PROPERTIES: &[&str] = &[
    "API_INTEGRATION",
    "EXTERNAL_ACCESS_INTEGRATIONS",
    "SECRETS",
    "SERVICE",
];

/// Snowflake function and procedure string properties, and string list properties, whose values may also be bare words.
pub(crate) const SNOWFLAKE_ROUTINE_WORD_PROPERTIES: &[&str] =
    &["COMMENT", "HANDLER", "IMPORTS", "TARGET_PATH"];

/// Snowflake function and procedure properties whose value is a `( '<key>' = <name> , ... )` map.
pub(crate) const SNOWFLAKE_ROUTINE_NAME_MAP_PROPERTIES: &[&str] = &["SECRETS"];

/// Snowflake function and procedure properties whose value is a `( '<key>' = '<value>' , ... )` map.
pub(crate) const SNOWFLAKE_ROUTINE_TEXT_MAP_PROPERTIES: &[&str] = &["HEADERS"];
/// The privileges a Snowflake GRANT or REVOKE names besides ALL, CREATE SNOWFLAKE classes and USE AI FUNCTION.
pub(crate) const SNOWFLAKE_GRANT_PRIVILEGES: &[&str] = &[
    "ADD SEARCH OPTIMIZATION",
    "APPLY",
    "APPLY AGGREGATION POLICY",
    "APPLY AUTHENTICATION POLICY",
    "APPLY CONTACT",
    "APPLY JOIN POLICY",
    "APPLY MAINTENANCE POLICY",
    "APPLY MASKING POLICY",
    "APPLY PACKAGES POLICY",
    "APPLY PASSWORD POLICY",
    "APPLY PROJECTION POLICY",
    "APPLY ROW ACCESS POLICY",
    "APPLY SESSION POLICY",
    "APPLY STORAGE LIFECYCLE POLICY",
    "APPLY TAG",
    "APPLYBUDGET",
    "ATTACH",
    "ATTACH LISTING",
    "ATTACH POLICY",
    "AUDIT",
    "BIND SERVICE",
    "BIND SERVICE ENDPOINT",
    "CREATE",
    "CREATE ACCOUNT",
    "CREATE AGENT",
    "CREATE AGGREGATION POLICY",
    "CREATE ALERT",
    "CREATE APPLICATION",
    "CREATE APPLICATION PACKAGE",
    "CREATE APPLICATION SERVICE",
    "CREATE ARTIFACT REPOSITORY",
    "CREATE AUTHENTICATION POLICY",
    "CREATE COMPUTE POOL",
    "CREATE CONTACT",
    "CREATE CORTEX SEARCH SERVICE",
    "CREATE DATA METRIC FUNCTION",
    "CREATE DATABASE",
    "CREATE DATABASE ROLE",
    "CREATE DATASET",
    "CREATE DBT PROJECT",
    "CREATE DCM PROJECT",
    "CREATE DYNAMIC TABLE",
    "CREATE EVENT TABLE",
    "CREATE EXPERIMENT",
    "CREATE EXTERNAL TABLE",
    "CREATE EXTERNAL VOLUME",
    "CREATE FAILOVER GROUP",
    "CREATE FILE FORMAT",
    "CREATE FUNCTION",
    "CREATE GATEWAY",
    "CREATE GIT REPOSITORY",
    "CREATE HYBRID TABLE",
    "CREATE ICEBERG TABLE",
    "CREATE IMAGE REPOSITORY",
    "CREATE INTEGRATION",
    "CREATE INTERACTIVE TABLE",
    "CREATE LISTING",
    "CREATE MAINTENANCE POLICY",
    "CREATE MASKING POLICY",
    "CREATE MATERIALIZED VIEW",
    "CREATE MCP SERVER",
    "CREATE MODEL",
    "CREATE NETWORK POLICY",
    "CREATE NETWORK RULE",
    "CREATE NOTEBOOK",
    "CREATE ONLINE FEATURE TABLE",
    "CREATE ORGANIZATION",
    "CREATE ORGANIZATION LISTING",
    "CREATE ORGANIZATION PROFILE",
    "CREATE PACKAGES POLICY",
    "CREATE PASSWORD POLICY",
    "CREATE PIPE",
    "CREATE PREVIEW APPLICATION",
    "CREATE PRIVACY POLICY",
    "CREATE PROCEDURE",
    "CREATE PROJECTION POLICY",
    "CREATE REPLICATION GROUP",
    "CREATE ROLE",
    "CREATE ROW ACCESS POLICY",
    "CREATE SCHEMA",
    "CREATE SECRET",
    "CREATE SEMANTIC VIEW",
    "CREATE SEQUENCE",
    "CREATE SERVICE",
    "CREATE SESSION POLICY",
    "CREATE SHARE",
    "CREATE SNAPSHOT",
    "CREATE SNAPSHOT POLICY",
    "CREATE SNAPSHOT SET",
    "CREATE STAGE",
    "CREATE STORAGE LIFECYCLE POLICY",
    "CREATE STREAM",
    "CREATE STREAMLIT",
    "CREATE TABLE",
    "CREATE TAG",
    "CREATE TASK",
    "CREATE TYPE",
    "CREATE USER",
    "CREATE VIEW",
    "CREATE WAREHOUSE",
    "CREATE WORKSPACE",
    "DELETE",
    "DEVELOP",
    "EVOLVE",
    "EVOLVE SCHEMA",
    "EXECUTE",
    "EXECUTE ALERT",
    "EXECUTE DATA METRIC FUNCTION",
    "EXECUTE MANAGED ALERT",
    "EXECUTE MANAGED TASK",
    "EXECUTE TASK",
    "FAILOVER",
    "IMPERSONATE",
    "IMPORT",
    "IMPORT ORGANIZATION LISTING",
    "IMPORT SHARE",
    "IMPORTED PRIVILEGES",
    "INSERT",
    "INSTALL",
    "MANAGE",
    "MANAGE ACCOUNT SUPPORT",
    "MANAGE ACCOUNT SUPPORT CASES",
    "MANAGE APPLICATION",
    "MANAGE APPLICATION SPECIFICATIONS",
    "MANAGE EVENT SHARING",
    "MANAGE GRANTS",
    "MANAGE LISTING AUTO FULFILLMENT",
    "MANAGE ORGANIZATION SUPPORT CASES",
    "MANAGE RELEASES",
    "MANAGE SHARE",
    "MANAGE SHARE TARGET",
    "MANAGE USER SUPPORT CASES",
    "MANAGE VERSIONS",
    "MANAGE VISIBILITY",
    "MANAGE WAREHOUSES",
    "MODIFY",
    "MODIFY LOG LEVEL",
    "MODIFY PROGRAMMATIC AUTHENTICATION METHODS",
    "MODIFY SESSION LOG LEVEL",
    "MODIFY SESSION TRACE LEVEL",
    "MODIFY TRACE LEVEL",
    "MONITOR",
    "MONITOR CORTEX EXTENSIONS",
    "MONITOR EXECUTION",
    "MONITOR SECURITY",
    "MONITOR USAGE",
    "OPERATE",
    "OVERRIDE SHARE",
    "OVERRIDE SHARE RESTRICTIONS",
    "OWNERSHIP",
    "PURCHASE DATA EXCHANGE",
    "PURCHASE DATA EXCHANGE LISTING",
    "READ",
    "READ ERROR TABLE",
    "READ SESSION",
    "READ UNREDACTED ERROR TABLE",
    "REFERENCES",
    "REFERENCE_USAGE",
    "REPLICATE",
    "RESOLVE",
    "RESOLVE ALL",
    "SELECT",
    "SELECT ERROR TABLE",
    "SELECT TABLE",
    "TRUNCATE",
    "UPDATE",
    "USAGE",
    "USE AI FUNCTIONS",
    "USE_ANY_ROLE",
    "WRITE",
];

/// The object kinds a Snowflake GRANT or REVOKE of privileges names before its object.
pub(crate) const SNOWFLAKE_GRANT_KINDS: &[&str] = &[
    "AGENT",
    "AGGREGATION POLICY",
    "ALERT",
    "APPLICATION",
    "APPLICATION PACKAGE",
    "APPLICATION SERVICE",
    "ARTIFACT REPOSITORY",
    "AUTHENTICATION POLICY",
    "COMPUTE POOL",
    "CONNECTION",
    "CONTACT",
    "CORTEX SEARCH SERVICE",
    "DATA EXCHANGE LISTING",
    "DATA METRIC FUNCTION",
    "DATABASE",
    "DATASET",
    "DBT PROJECT",
    "DCM PROJECT",
    "DYNAMIC TABLE",
    "EVENT TABLE",
    "EXPERIMENT",
    "EXTERNAL TABLE",
    "EXTERNAL VOLUME",
    "FAILOVER GROUP",
    "FILE FORMAT",
    "FUNCTION",
    "GATEWAY",
    "GIT REPOSITORY",
    "HYBRID TABLE",
    "ICEBERG TABLE",
    "IMAGE REPOSITORY",
    "INTEGRATION",
    "INTERACTIVE TABLE",
    "JOIN POLICY",
    "LISTING",
    "MAINTENANCE POLICY",
    "MASKING POLICY",
    "MATERIALIZED VIEW",
    "MCP SERVER",
    "MODEL",
    "NETWORK POLICY",
    "NETWORK RULE",
    "NOTEBOOK",
    "ONLINE FEATURE TABLE",
    "ORGANIZATION LISTING",
    "PACKAGES POLICY",
    "PASSWORD POLICY",
    "PIPE",
    "PRIVACY POLICY",
    "PROCEDURE",
    "PROJECTION POLICY",
    "REPLICATION GROUP",
    "RESOURCE MONITOR",
    "ROW ACCESS POLICY",
    "SCHEMA",
    "SECRET",
    "SEMANTIC VIEW",
    "SEQUENCE",
    "SERVICE",
    "SESSION POLICY",
    "SNAPSHOT",
    "SNAPSHOT POLICY",
    "SNAPSHOT SET",
    "STAGE",
    "STORAGE LIFECYCLE POLICY",
    "STREAM",
    "STREAMLIT",
    "TABLE",
    "TAG",
    "TASK",
    "TYPE",
    "USER",
    "VIEW",
    "WAREHOUSE",
    "WORKSPACE",
];

/// The Snowflake kinds a CREATE defines only by `<property> = <value>` lists.
pub(crate) const SNOWFLAKE_PROPERTY_KINDS: &[&str] = &[
    "ACCOUNT",
    "API INTEGRATION",
    "APPLICATION ROLE",
    "AUTHENTICATION POLICY",
    "BACKUP POLICY",
    "CATALOG INTEGRATION",
    "DATA MOVEMENT POLICY",
    "EXTERNAL ACCESS INTEGRATION",
    "FAILOVER GROUP",
    "FEATURE POLICY",
    "INGRESS POLICY",
    "MAINTENANCE POLICY",
    "MANAGED ACCOUNT",
    "MULTI PARTY APPROVAL POLICY",
    "NETWORK POLICY",
    "NETWORK RULE",
    "NOTIFICATION INTEGRATION",
    "ORGANIZATION ACCOUNT",
    "ORGANIZATION USER",
    "PACKAGES POLICY",
    "PASSWORD POLICY",
    "REPLICATION GROUP",
    "RESOURCE MONITOR",
    "ROLE",
    "SECRET",
    "SECURITY INTEGRATION",
    "SESSION POLICY",
    "SHARE",
    "SNAPSHOT POLICY",
    "STORAGE INTEGRATION",
    "USER",
];

/// The Snowflake policy kinds a CREATE defines by a signature, a return type and a body.
pub(crate) const SNOWFLAKE_BODY_POLICY_KINDS: &[&str] = &[
    "AGGREGATION POLICY",
    "JOIN POLICY",
    "MASKING POLICY",
    "PRIVACY POLICY",
    "PROJECTION POLICY",
    "ROW ACCESS POLICY",
    "STORAGE LIFECYCLE POLICY",
];
