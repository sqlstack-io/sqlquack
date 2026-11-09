//! Operator precedence levels of `valueExpression`, lowest first, and the grammar's word sets.

use sqlquack_syntax::expressions::types::models::IntervalUnit;

/// A statement whose shared tree nests deeper than this is refused after it parses.
pub(crate) const MAX_TREE_DEPTH: usize = 8000;

/// `comparisonOperator`.
pub(crate) const COMPARE: u8 = 1;
/// `|` (bitwise or).
pub(crate) const BIT_OR: u8 = 2;
/// `^`.
pub(crate) const BIT_XOR: u8 = 3;
/// `&`.
pub(crate) const BIT_AND: u8 = 4;
/// `shiftOperator`.
pub(crate) const SHIFT: u8 = 5;
/// `+`, `-` and `||`.
pub(crate) const ADD: u8 = 6;
/// `*`, `/`, `%` and `DIV`.
pub(crate) const MULTIPLY: u8 = 7;

/// Words after which a `|` is the pipe operator rather than bitwise or (`isOperatorPipeStart`).
pub(crate) const PIPE_STARTS: [&str; 31] = [
    "SELECT",
    "EXTEND",
    "SET",
    "DROP",
    "AS",
    "WHERE",
    "PIVOT",
    "UNPIVOT",
    "TABLESAMPLE",
    "INNER",
    "CROSS",
    "LEFT",
    "RIGHT",
    "FULL",
    "NATURAL",
    "SEMI",
    "ANTI",
    "JOIN",
    "UNION",
    "EXCEPT",
    "MINUS",
    "INTERSECT",
    "ORDER",
    "CLUSTER",
    "DISTRIBUTE",
    "SORT",
    "LIMIT",
    "OFFSET",
    "AGGREGATE",
    "WINDOW",
    "LATERAL",
];

/// Words that begin a query (`queryPrimary` or `ctes`): a parenthesis holding one is a subquery.
pub(crate) const QUERY_STARTS: [&str; 5] = ["SELECT", "WITH", "VALUES", "FROM", "TABLE"];

/// Words beginning a clause that may follow a select item, table or pivot: never a bare alias.
pub(crate) const CLAUSE_WORDS: [&str; 40] = [
    "APPROX",
    "EXACT",
    "FROM",
    "TABLE",
    "WHERE",
    "GROUP",
    "HAVING",
    "ORDER",
    "LIMIT",
    "OFFSET",
    "WINDOW",
    "QUALIFY",
    "UNION",
    "EXCEPT",
    "MINUS",
    "INTERSECT",
    "LATERAL",
    "CLUSTER",
    "DISTRIBUTE",
    "SORT",
    "SELECT",
    "FOR",
    "PIVOT",
    "UNPIVOT",
    "TABLESAMPLE",
    "SET",
    "INSERT",
    "VERSION",
    "TIMESTAMP",
    "SYSTEM_VERSION",
    "SYSTEM_TIME",
    "WITH",
    "WATERMARK",
    "IDENTIFIED",
    "REPLACE",
    "BY",
    "PARTITION",
    "CHANGES",
    "OPTIONS",
    "VALUES",
];

/// Clause words that begin their clause only before the word paired with them here.
pub(crate) const PAIRED_CLAUSE_WORDS: [(&str, &[&str]); 13] = [
    ("APPROX", &["NEAREST"]),
    ("EXACT", &["NEAREST"]),
    ("GROUP", &["BY"]),
    ("ORDER", &["BY"]),
    ("CLUSTER", &["BY"]),
    ("DISTRIBUTE", &["BY"]),
    ("SORT", &["BY"]),
    ("LATERAL", &["VIEW"]),
    ("VERSION", &["AS"]),
    ("TIMESTAMP", &["AS"]),
    ("SYSTEM_VERSION", &["AS"]),
    ("SYSTEM_TIME", &["AS"]),
    ("BY", &["NAME"]),
];

/// The words a statement may begin with; any other first word is a syntax error.
pub(crate) const STATEMENT_STARTS: [&str; 60] = [
    "alter", "analyze", "apply", "begin", "cache", "call", "case", "clear", "close", "comment",
    "commit", "convert", "copy", "create", "declare", "delete", "deny", "desc", "describe", "drop",
    "execute", "explain", "fetch", "for", "fsck", "generate", "get", "grant", "if", "insert",
    "iterate", "leave", "list", "load", "loop", "merge", "msck", "open", "optimize", "put",
    "refresh", "remove", "reorg", "repair", "repeat", "replace", "reset", "restore", "revoke",
    "rollback", "set", "show", "signal", "sync", "truncate", "uncache", "undrop", "update", "use",
    "vacuum",
];

/// Further statement starts: `ADD` (resources) and the scripting words `WHILE` and `RESIGNAL`.
pub(crate) const STATEMENT_STARTS_MORE: [&str; 3] = ["add", "while", "resignal"];

/// `datetimeUnit`, which `TIMESTAMPADD` and `TIMESTAMPDIFF` take as their first argument.
pub(crate) const DATETIME_UNITS: [&str; 11] = [
    "YEAR",
    "QUARTER",
    "MONTH",
    "WEEK",
    "DAY",
    "DAYOFYEAR",
    "HOUR",
    "MINUTE",
    "SECOND",
    "MILLISECOND",
    "MICROSECOND",
];

/// The `currentLike` words, valid without parentheses.
pub(crate) const CURRENT_LIKE: [&str; 7] = [
    "CURRENT_DATE",
    "CURRENT_TIMESTAMP",
    "CURRENT_USER",
    "USER",
    "SESSION_USER",
    "CURRENT_TIME",
    "CURRENT_PATH",
];

/// `literalType` keywords before a string: a typed literal.
pub(crate) const LITERAL_TYPES: [&str; 6] = [
    "DATE",
    "TIME",
    "TIMESTAMP",
    "TIMESTAMP_LTZ",
    "TIMESTAMP_NTZ",
    "X",
];

/// The names `TIMESTAMPADD` and `TIMESTAMPDIFF` go by.
pub(crate) const TIMESTAMP_ADD_NAMES: [&str; 3] = ["TIMESTAMPADD", "DATEADD", "DATE_ADD"];
pub(crate) const TIMESTAMP_DIFF_NAMES: [&str; 4] =
    ["TIMESTAMPDIFF", "DATEDIFF", "DATE_DIFF", "TIMEDIFF"];

/// `unitInMultiUnits`: interval units, singular and plural, each with its shared-tree unit name.
pub(crate) const INTERVAL_UNITS: [(&str, IntervalUnit); 20] = [
    ("NANOSECOND", IntervalUnit::Nanosecond),
    ("NANOSECONDS", IntervalUnit::Nanosecond),
    ("MICROSECOND", IntervalUnit::Microsecond),
    ("MICROSECONDS", IntervalUnit::Microsecond),
    ("MILLISECOND", IntervalUnit::Millisecond),
    ("MILLISECONDS", IntervalUnit::Millisecond),
    ("SECOND", IntervalUnit::Second),
    ("SECONDS", IntervalUnit::Second),
    ("MINUTE", IntervalUnit::Minute),
    ("MINUTES", IntervalUnit::Minute),
    ("HOUR", IntervalUnit::Hour),
    ("HOURS", IntervalUnit::Hour),
    ("DAY", IntervalUnit::Day),
    ("DAYS", IntervalUnit::Day),
    ("WEEK", IntervalUnit::Week),
    ("WEEKS", IntervalUnit::Week),
    ("MONTH", IntervalUnit::Month),
    ("MONTHS", IntervalUnit::Month),
    ("YEAR", IntervalUnit::Year),
    ("YEARS", IntervalUnit::Year),
];

/// `unitInUnitToUnit`: the units an `INTERVAL ... TO ...` range may name.
pub(crate) const UNIT_TO_UNITS: [&str; 6] = ["SECOND", "MINUTE", "HOUR", "DAY", "MONTH", "YEAR"];

/// The name the tree gives the bucketing, cube and grouping-set calls of `GROUP BY`.
pub(crate) const ROLLUP: &str = "ROLLUP";
pub(crate) const CUBE: &str = "CUBE";
pub(crate) const GROUPING_SETS: &str = "GROUPING SETS";

/// The function name an `IDENTIFIER (expr)` clause becomes, as lineage reads it.
pub(crate) const IDENTIFIER_CLAUSE: &str = "IDENTIFIER";

/// The object words a `CREATE` the shared tree has no node for may name; any other word is refused.
pub(crate) const CREATE_KINDS: [&str; 29] = [
    "BLOOMFILTER",
    "CATALOG",
    "CLEAN",
    "CONNECTION",
    "CREDENTIAL",
    "DATABASE",
    "EXTERNAL",
    "FLOW",
    "FOREIGN",
    "FUNCTION",
    "GOVERNED",
    "GROUP",
    "INDEX",
    "MATERIALIZED",
    "NAMESPACE",
    "ONLINE",
    "POLICY",
    "PROCEDURE",
    "PROVIDER",
    "RECIPIENT",
    "ROLE",
    "SCHEMA",
    "SERVER",
    "SHARE",
    "STORAGE",
    "STREAMING",
    "USER",
    "VARIABLE",
    "VOLUME",
];

/// The object words a `DROP` the shared tree has no node for may name; any other word is refused.
pub(crate) const DROP_KINDS: [&str; 31] = [
    "BLOOMFILTER",
    "CATALOG",
    "CLEAN",
    "CONNECTION",
    "CREDENTIAL",
    "DATABASE",
    "EXTERNAL",
    "FEATURE",
    "FLOW",
    "FOREIGN",
    "FUNCTION",
    "GOVERNED",
    "GROUP",
    "INDEX",
    "MATERIALIZED",
    "NAMESPACE",
    "ONLINE",
    "POLICY",
    "PROCEDURE",
    "PROVIDER",
    "RECIPIENT",
    "ROLE",
    "SCHEMA",
    "SERVER",
    "SHARE",
    "STORAGE",
    "STREAMING",
    "TEMPORARY",
    "USER",
    "VARIABLE",
    "VOLUME",
];

/// The object words an `ALTER` other than of a table, view or schema may name, kept as a gap.
pub(crate) const ALTER_KINDS: [&str; 19] = [
    "CATALOG",
    "CLEAN",
    "CONNECTION",
    "CREDENTIAL",
    "EXTERNAL",
    "FLOW",
    "FOREIGN",
    "FUNCTION",
    "GOVERNED",
    "GROUP",
    "MATERIALIZED",
    "POLICY",
    "PROCEDURE",
    "PROVIDER",
    "RECIPIENT",
    "SHARE",
    "STORAGE",
    "STREAMING",
    "VOLUME",
];

/// Words that begin an `alterColumnAction`, so a name before one is no Hive `CHANGE` rename.
pub(crate) const ALTER_COLUMN_WORDS: [&str; 7] =
    ["TYPE", "COMMENT", "FIRST", "AFTER", "SET", "DROP", "SYNC"];

/// The object kinds `COMMENT ON` takes besides a schema's other names.
pub(crate) const COMMENT_KINDS: [&str; 9] = [
    "CATALOG",
    "COLUMN",
    "CONNECTION",
    "PROVIDER",
    "RECIPIENT",
    "SCHEMA",
    "SHARE",
    "TABLE",
    "VOLUME",
];

/// The object kinds `DESCRIBE` takes besides relations, queries, functions and schemas.
pub(crate) const DESCRIBE_KINDS: [&str; 8] = [
    "CATALOG",
    "CONNECTION",
    "CREDENTIAL",
    "LOCATION",
    "PROVIDER",
    "RECIPIENT",
    "SHARE",
    "VOLUME",
];

/// `EXPLAIN`'s modes.
pub(crate) const EXPLAIN_MODES: [&str; 5] = ["LOGICAL", "FORMATTED", "EXTENDED", "CODEGEN", "COST"];

/// `pathElement`'s keywords.
pub(crate) const PATH_ELEMENTS: [&str; 5] = [
    "DEFAULT_PATH",
    "SYSTEM_PATH",
    "PATH",
    "CURRENT_DATABASE",
    "CURRENT_SCHEMA",
];

/// The securable kinds `GRANT` and `REVOKE` name, longest first, and whether a name is optional.
pub(crate) const SECURABLE_KINDS: [(&[&str], bool); 18] = [
    (&["MATERIALIZED", "VIEW"], false),
    (&["EXTERNAL", "LOCATION"], false),
    (&["STORAGE", "CREDENTIAL"], false),
    (&["SERVICE", "CREDENTIAL"], false),
    (&["ANONYMOUS", "FUNCTION"], true),
    (&["ANY", "FILE"], true),
    (&["CATALOG"], true),
    (&["SCHEMA"], false),
    (&["DATABASE"], false),
    (&["TABLE"], false),
    (&["VIEW"], false),
    (&["FUNCTION"], false),
    (&["PROCEDURE"], false),
    (&["VOLUME"], false),
    (&["SHARE"], false),
    (&["CONNECTION"], false),
    (&["CREDENTIAL"], false),
    (&["MODEL"], false),
];

/// Interval fields a year-month interval type may start with.
pub(crate) const YEAR_MONTH: [&str; 2] = ["YEAR", "MONTH"];
/// Interval fields a day-time interval type may start with.
pub(crate) const DAY_TIME: [&str; 4] = ["DAY", "HOUR", "MINUTE", "SECOND"];
/// The field a year-month interval type may end with.
pub(crate) const MONTH: &str = "MONTH";
/// The `INT` spelling the tree marks.
pub(crate) const INTEGER_SPELLING: &str = "INTEGER";
/// The `FLOAT` spelling the tree marks.
pub(crate) const REAL_SPELLING: &str = "REAL";
/// The true boolean literal.
pub(crate) const TRUE: &str = "TRUE";
/// The cast call that yields null instead of failing.
pub(crate) const TRY_CAST: &str = "TRY_CAST";
/// The case-insensitive form of `LIKE`.
pub(crate) const ILIKE: &str = "ILIKE";
/// The string concatenation operator.
pub(crate) const CONCAT_OPERATOR: &str = "||";
/// The left shift operator.
pub(crate) const SHIFT_LEFT_OPERATOR: &str = "<<";
/// The fewest parameters a parenthesized lambda parameter list holds.
pub(crate) const MIN_LAMBDA_PARAMETERS: usize = 2;
/// The index of a date's day segment, after its year and month.
pub(crate) const DAY_SEGMENT: usize = 2;
/// The clause word that begins a clause only before `(`.
pub(crate) const MATCH_RECOGNIZE: &str = "MATCH_RECOGNIZE";
/// The word that ends a clause word's alias reading when it follows the word.
pub(crate) const FOR: &str = "FOR";
/// The repeat marker of a form pattern whose items are comma-separated.
pub(crate) const COMMA_SEPARATED_REPEAT: &str = ",...";
/// The tag action that sets tags, rather than unsetting them.
pub(crate) const SET_TAGS: &str = "SET";
/// The repair gap written with `MSCK`.
pub(crate) const MSCK: &str = "msck";
/// The documented form whose `AUTO CDC` flow is checked further.
pub(crate) const CREATE_STREAMING_TABLE: &str = "create_streaming_table";
/// The documented form whose source and target names are checked further.
pub(crate) const SYNC: &str = "sync";
/// The routine language that takes no handler.
pub(crate) const SQL_LANGUAGE: &str = "SQL";
/// The described object a string may name.
pub(crate) const CATALOG: &str = "CATALOG";
/// The `SHOW` word naming functions.
pub(crate) const FUNCTIONS: &str = "FUNCTIONS";
/// The fewest parts a `SET PATH` schema reference names.
pub(crate) const MIN_PATH_PARTS: usize = 2;
/// The fewest characters a backquoted configuration value holds: its two backquotes.
pub(crate) const MIN_BACKQUOTED_LENGTH: usize = 2;
/// `SET -v`, which lists every configuration with its meaning.
pub(crate) const SET_VERBOSE: &str = "-v";
/// The resource statement that adds a resource, rather than listing them.
pub(crate) const ADD_RESOURCE: &str = "ADD";
