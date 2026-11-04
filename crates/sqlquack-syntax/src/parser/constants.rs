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

/// The text "DEFINER", as comparisons test it.
pub(crate) const DEFINER: &str = "DEFINER";

/// The text "ENGINE", as comparisons test it.
pub(crate) const ENGINE: &str = "ENGINE";

/// The text "EXCLUDE", as comparisons test it.
pub(crate) const EXCLUDE: &str = "EXCLUDE";

/// The text "FILE", as comparisons test it.
pub(crate) const FILE: &str = "FILE";

/// The text "FIRST_VALUE", as comparisons test it.
pub(crate) const FIRST_VALUE: &str = "FIRST_VALUE";

/// The text "FUNCTION", as comparisons test it.
pub(crate) const FUNCTION: &str = "FUNCTION";

/// The text "GROUP", as comparisons test it.
pub(crate) const GROUP: &str = "GROUP";

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

/// The argument count of a two-argument call.
pub(crate) const BINARY_ARITY: usize = 2;

/// The argument count of a three-argument call.
pub(crate) const TERNARY_ARITY: usize = 3;

/// The parts of a two-part qualified name, `schema.name`.
pub(crate) const QUALIFIED_NAME_PARTS: usize = 2;

/// How far back the token before the previous one is.
pub(crate) const SECOND_PREVIOUS_OFFSET: usize = 2;

/// The most terms a boolean chain can have and still be left as parsed.
pub(crate) const BALANCED_CHAIN_MAX_TERMS: usize = 2;

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
pub(crate) const SQLITE_RANGE_FOLLOWERS: [crate::tokens::models::TokenType; 9] = [
    crate::tokens::models::TokenType::Not,
    crate::tokens::models::TokenType::In,
    crate::tokens::models::TokenType::Is,
    crate::tokens::models::TokenType::Between,
    crate::tokens::models::TokenType::Like,
    crate::tokens::models::TokenType::ILike,
    crate::tokens::models::TokenType::RLike,
    crate::tokens::models::TokenType::Glob,
    crate::tokens::models::TokenType::Match,
];

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
