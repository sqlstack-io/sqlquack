//! Operator precedence levels of `valueExpression`, lowest first, and the grammar's word sets.

use sqlquack_syntax::expressions::types::models::IntervalUnit;

/// Pattern keywords that end their clause, unless an optional part or choice opened after them goes on.
pub(crate) const FLAG_PHRASES: &[&str] = &[
    "IF EXISTS",
    "IF NOT EXISTS",
    "IGNORE NULL UPDATES",
    "ON METASTORE",
    "FOR TABLES",
];

/// The words written for SCHEMA that the tree keeps as written.
pub(crate) const SCHEMA_SYNONYMS: &[&str] = &["DATABASE", "NAMESPACE"];

/// The word `USE NAMESPACE` writes for `USE DATABASE`.
pub(crate) const NAMESPACE: &str = "NAMESPACE";

/// A statement whose shared tree nests deeper than this is refused after it parses.
pub(crate) const MAX_TREE_DEPTH: usize = 8000;

/// Levels a statement may wrap around a value kept where a gap used to drop it; a deeper value stays a gap.
pub(crate) const KEPT_VALUE_WRAPPERS: usize = 64;

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

/// The `currentLike` words and `LOCALTIME`, which the warehouse also reads bare as the call.
pub(crate) const CURRENT_LIKE: [&str; 8] = [
    "CURRENT_DATE",
    "CURRENT_TIMESTAMP",
    "CURRENT_USER",
    "USER",
    "SESSION_USER",
    "CURRENT_TIME",
    "CURRENT_PATH",
    "LOCALTIME",
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

/// The units a `RETAIN DROPPED` period may be written in.
pub(crate) const RETENTION_UNITS: [&str; 6] = ["HOUR", "HOURS", "DAY", "DAYS", "WEEK", "WEEKS"];
/// `unitInUnitToUnit`: the units an `INTERVAL ... TO ...` range may name.
pub(crate) const UNIT_TO_UNITS: [&str; 6] = ["SECOND", "MINUTE", "HOUR", "DAY", "MONTH", "YEAR"];

/// The name the tree gives the bucketing, cube and grouping-set calls of `GROUP BY`.
pub(crate) const ROLLUP: &str = "ROLLUP";
pub(crate) const CUBE: &str = "CUBE";
pub(crate) const GROUPING_SETS: &str = "GROUPING SETS";

/// The scope kind of a `SHOW` that lists a schema's objects, which an `IDENTIFIER (...)` may name.
pub(crate) const SCHEMA_SCOPE: &str = "SCHEMA";

/// The digits of an `@yyyyMMddHHmmssSSS` timestamp after a Delta table name.
pub(crate) const AT_TIMESTAMP_DIGITS: usize = 17;

/// The function name an `IDENTIFIER (expr)` clause becomes, as lineage reads it.
pub(crate) const IDENTIFIER_CLAUSE: &str = "IDENTIFIER";

/// The object words a `CREATE` the shared tree has no node for may name; any other word is refused.
pub(crate) const CREATE_KINDS: [&str; 28] = [
    "BLOOMFILTER",
    "CATALOG",
    "CLEAN",
    "CONNECTION",
    "CREDENTIAL",
    "DATABASE",
    "EXTERNAL",
    "FLOW",
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
pub(crate) const SECURABLE_KINDS: [(&[&str], bool); 20] = [
    (&["MATERIALIZED", "VIEW"], false),
    (&["EXTERNAL", "LOCATION"], false),
    (&["EXTERNAL", "METADATA"], false),
    (&["STORAGE", "CREDENTIAL"], false),
    (&["SERVICE", "CREDENTIAL"], false),
    (&["ANONYMOUS", "FUNCTION"], true),
    (&["ANY", "FILE"], true),
    (&["CATALOG"], true),
    (&["METASTORE"], true),
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

/// The securable kinds that never take a name.
pub(crate) const NAMELESS_SECURABLES: [&str; 3] = ["ANY FILE", "ANONYMOUS FUNCTION", "METASTORE"];

/// The language whose handler needs an `ENVIRONMENT` naming its dependencies.
pub(crate) const SCALA_LANGUAGE: &str = "SCALA";

/// Interval fields a year-month interval type may start with.
pub(crate) const YEAR_MONTH: [&str; 2] = ["YEAR", "MONTH"];
/// Interval fields a day-time interval type may start with.
pub(crate) const DAY_TIME: [&str; 4] = ["DAY", "HOUR", "MINUTE", "SECOND"];
/// The field a year-month interval type may end with.
pub(crate) const MONTH: &str = "MONTH";
/// The `INT` spelling the tree marks.
pub(crate) const INTEGER_SPELLING: &str = "INTEGER";
/// The set operator Databricks reads as `EXCEPT`, kept as written.
pub(crate) const MINUS: &str = "MINUS";
/// The `FLOAT` spelling the tree marks.
pub(crate) const REAL_SPELLING: &str = "REAL";
/// The timestamp type spelling with the local time zone.
pub(crate) const LOCAL_TIMESTAMP: &str = "TIMESTAMP_LTZ";
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
/// `SET TAG` and `UNSET TAG`, with the patterns that read their securable by its kind.
pub(crate) const TAG_FORMS: [(&str, &str); 2] = [
    ("set_tag", "SET TAG ON tag_target id [ = tag_value ]"),
    ("unset_tag", "UNSET TAG ON tag_target id"),
];
/// The words that open a tag's securable kind; written before a name, they are that kind.
pub(crate) const TAG_KINDS: [&str; 11] = [
    "CATALOG",
    "COLUMN",
    "DATABASE",
    "EXTERNAL",
    "FUNCTION",
    "PROCEDURE",
    "SCHEMA",
    "SECRET",
    "TABLE",
    "VIEW",
    "VOLUME",
];
/// The routine language that takes no handler.
pub(crate) const SQL_LANGUAGE: &str = "SQL";
/// The described object a string may name.
pub(crate) const CATALOG: &str = "CATALOG";
/// The object kind whose DESCRIBE target is a table-like name with parts.
pub(crate) const VOLUME: &str = "VOLUME";
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
/// `CACHE SELECT` as its page writes it.
pub(crate) const CACHE_SELECT: &str = "CACHE SELECT expr ,... FROM name [ WHERE expr ]";
/// `ADD` of resources: the kind, then any resources.
pub(crate) const ADD_RESOURCES: &str =
    "ADD { FILE | FILES | JAR | JARS | ARCHIVE | ARCHIVES } [ resource ... ]";
/// `LIST` of resources: the kind, then any resources.
pub(crate) const LIST_RESOURCES: &str =
    "LIST { FILE | FILES | JAR | JARS | ARCHIVE | ARCHIVES } [ resource ... ]";

/// `createTempViewUsing` as its page writes it.
pub(crate) const TEMPORARY_VIEW_USING: &str = "CREATE [ OR REPLACE ] [ GLOBAL ] { TEMPORARY | TEMP } \
     VIEW name [ ( column ,... ) ] USING name [ OPTIONS props ]";
/// `DROP TEMPORARY TABLE`.
pub(crate) const DROP_TEMPORARY_TABLE: &str =
    "DROP { TEMPORARY | TEMP } TABLE [ IF EXISTS ] table [ PURGE ]";
/// `DROP TABLE ... FORCE`.
pub(crate) const DROP_TABLE_FORCE: &str = "DROP TABLE [ IF EXISTS ] table [ PURGE ] FORCE";
/// `DROP TEMPORARY FUNCTION`.
pub(crate) const DROP_TEMPORARY_FUNCTION: &str =
    "DROP { TEMPORARY | TEMP } FUNCTION [ IF EXISTS ] table";
/// `UNDROP ... WITH ID`.
pub(crate) const UNDROP_WITH_ID: &str = "UNDROP { TABLE | VOLUME | MATERIALIZED VIEW } WITH ID str";

/// Forms read as trees that the formatter and lint still refuse as gaps until their layout is reviewed; any future Databricks translator or renderer must handle each of these, and every key of `KEYED_ACTIONS`, or refuse it.
pub(crate) const TREED_FORMS: [&str; 38] = [
    "add",
    "list",
    "restore",
    "deny",
    "cache_select",
    "refresh",
    "describe_column",
    "show_partition",
    "row_format",
    "stored_as",
    "constraint_validation",
    "column_mask",
    "row_filter",
    "owner",
    "managed",
    "serde",
    "table_collation",
    "recover_partitions",
    "replace_partitioning",
    "predictive_optimization",
    "rename_partition",
    "drop_primary_key",
    "drop_foreign_key",
    "drop_feature",
    "alter_column_position",
    "identity_sync",
    "column_tags",
    "schema_binding",
    "view_collation",
    "view_partitioned_on",
    "view_nocopy",
    "temporary_view_using",
    "drop_temporary_table",
    "drop_table_force",
    "drop_temporary_function",
    "undrop_with_id",
    "compound",
    "compound_cursor",
];

/// `ALTER TABLE ... OWNER TO`.
pub(crate) const OWNER_TO: &str = "OWNER TO";
/// The scripting statement that re-raises a condition, which may then be left out.
pub(crate) const RESIGNAL: &str = "RESIGNAL";
/// `ALTER TABLE ... SET OWNER TO`.
pub(crate) const SET_OWNER_TO: &str = "SET OWNER TO";
/// `ALTER TABLE ... SET ROW FILTER`.
pub(crate) const SET_ROW_FILTER: &str = "SET ROW FILTER";
/// `ALTER TABLE ... DROP ROW FILTER`.
pub(crate) const DROP_ROW_FILTER: &str = "DROP ROW FILTER";
/// `ALTER TABLE ... SET MANAGED`.
pub(crate) const SET_MANAGED: &str = "SET MANAGED";
/// `ALTER TABLE ... SET EXTERNAL`.
pub(crate) const SET_EXTERNAL: &str = "SET EXTERNAL";
/// `ALTER TABLE ... UNSET MANAGED`.
pub(crate) const UNSET_MANAGED: &str = "UNSET MANAGED";
/// `ALTER TABLE ... SET SERDE`.
pub(crate) const SET_SERDE: &str = "SET SERDE";
/// `ALTER TABLE ... SET SERDEPROPERTIES`.
pub(crate) const SET_SERDEPROPERTIES: &str = "SET SERDEPROPERTIES";
/// `ALTER TABLE ... DEFAULT COLLATION`.
pub(crate) const DEFAULT_COLLATION: &str = "DEFAULT COLLATION";
/// `ALTER TABLE ... RECOVER PARTITIONS`.
pub(crate) const RECOVER_PARTITIONS: &str = "RECOVER PARTITIONS";
/// `ALTER TABLE ... REPLACE PARTITIONED BY WITH CLUSTER BY`.
pub(crate) const REPLACE_PARTITIONING: &str = "REPLACE PARTITIONED BY WITH CLUSTER BY";
/// `ALTER TABLE ... { ENABLE | DISABLE | INHERIT } PREDICTIVE OPTIMIZATION`.
pub(crate) const PREDICTIVE_OPTIMIZATION: &str = "PREDICTIVE OPTIMIZATION";
/// `ALTER TABLE ... PARTITION (...) RENAME TO PARTITION (...)`.
pub(crate) const RENAME_PARTITION: &str = "RENAME TO PARTITION";
/// `ALTER TABLE ... DROP PRIMARY KEY`.
pub(crate) const DROP_PRIMARY_KEY: &str = "DROP PRIMARY KEY";
/// `ALTER TABLE ... DROP FOREIGN KEY`.
pub(crate) const DROP_FOREIGN_KEY: &str = "DROP FOREIGN KEY";
/// `ALTER TABLE ... DROP FEATURE`.
pub(crate) const DROP_FEATURE: &str = "DROP FEATURE";
/// `ALTER TABLE ... ALTER COLUMN c { FIRST | AFTER c2 }`.
pub(crate) const COLUMN_POSITION: &str = "COLUMN POSITION";
/// `ALTER TABLE ... ALTER COLUMN c SET MASK`.
pub(crate) const COLUMN_SET_MASK: &str = "COLUMN SET MASK";
/// `ALTER TABLE ... ALTER COLUMN c DROP MASK`.
pub(crate) const COLUMN_DROP_MASK: &str = "COLUMN DROP MASK";
/// `ALTER TABLE ... ALTER COLUMN c SYNC IDENTITY`.
pub(crate) const COLUMN_SYNC_IDENTITY: &str = "COLUMN SYNC IDENTITY";
/// `ALTER TABLE ... ALTER COLUMN c SET TAGS`.
pub(crate) const COLUMN_SET_TAGS: &str = "COLUMN SET TAGS";
/// `ALTER TABLE ... ALTER COLUMN c UNSET TAGS`.
pub(crate) const COLUMN_UNSET_TAGS: &str = "COLUMN UNSET TAGS";
/// The words `SET MANAGED` and `UNSET MANAGED` take to truncate Uniform history.
pub(crate) const TRUNCATE_UNIFORM_HISTORY: &str = "TRUNCATE UNIFORM HISTORY";

/// Keys of the `ALTER TABLE` actions with no shared variant, each a `SetProperty` entry; any future Databricks translator or renderer must handle every key or refuse it.
pub(crate) const KEYED_ACTIONS: [&str; 23] = [
    OWNER_TO,
    SET_OWNER_TO,
    SET_ROW_FILTER,
    DROP_ROW_FILTER,
    SET_MANAGED,
    SET_EXTERNAL,
    UNSET_MANAGED,
    SET_SERDE,
    SET_SERDEPROPERTIES,
    DEFAULT_COLLATION,
    RECOVER_PARTITIONS,
    REPLACE_PARTITIONING,
    PREDICTIVE_OPTIMIZATION,
    RENAME_PARTITION,
    DROP_PRIMARY_KEY,
    DROP_FOREIGN_KEY,
    DROP_FEATURE,
    COLUMN_POSITION,
    COLUMN_SET_MASK,
    COLUMN_DROP_MASK,
    COLUMN_SYNC_IDENTITY,
    COLUMN_SET_TAGS,
    COLUMN_UNSET_TAGS,
];

/// Words that take a parenthesised query in an expression, so `REPLACE ON` does not end before it.
pub(crate) const REPLACE_ON_QUERY_TAKERS: [&str; 5] = ["EXISTS", "IN", "ANY", "SOME", "ALL"];

/// The most keywords written before a JOIN, as in `NATURAL LEFT OUTER JOIN`.
pub(crate) const MAX_JOIN_PREFIX: usize = 4;
