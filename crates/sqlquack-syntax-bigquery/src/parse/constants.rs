//! Operator precedence levels of `valueExpression`, lowest first, and the grammar's word sets.

use sqlquack_syntax::expressions::types::models::IntervalUnit;

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
/// `+` and `-`.
pub(crate) const ADD: u8 = 6;
/// `||`, above `+` and below `*` in googlesql.tm.
pub(crate) const CONCAT: u8 = 7;
/// `*`, `/`, `%` and `DIV`.
pub(crate) const MULTIPLY: u8 = 8;

/// The right side of `IS UNKNOWN`, which GoogleSQL keeps apart from `IS NULL`.
pub(crate) const UNKNOWN_TEST: &str = "UNKNOWN";

/// The right side of `IS NOT UNKNOWN`.
pub(crate) const NOT_UNKNOWN_TEST: &str = "NOT UNKNOWN";

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

/// Words that begin a clause after a select item, a table or a pivot, so they are no bare alias.
pub(crate) const CLAUSE_WORDS: [&str; 41] = [
    "MATCH_RECOGNIZE",
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

/// The words a GoogleSQL statement or script statement begins with; any other first word is refused.
pub(crate) const STATEMENT_STARTS: [&str; 38] = [
    "abort", "alter", "analyze", "assert", "begin", "break", "call", "case", "clone", "commit",
    "continue", "create", "declare", "define", "delete", "describe", "drop", "execute", "explain",
    "export", "for", "grant", "if", "import", "iterate", "leave", "load", "loop", "raise",
    "rename", "repeat", "return", "revoke", "rollback", "set", "show", "start", "truncate",
];

/// Further statement starts: `UNDROP`, `WHILE`, `DESC` and the transaction words.
pub(crate) const STATEMENT_STARTS_MORE: [&str; 5] = ["undrop", "while", "desc", "end", "run"];

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
pub(crate) const CURRENT_LIKE: [&str; 8] = [
    "CURRENT_DATE",
    "CURRENT_DATETIME",
    "CURRENT_TIMESTAMP",
    "CURRENT_USER",
    "USER",
    "SESSION_USER",
    "CURRENT_TIME",
    "CURRENT_PATH",
];

/// Type names before a string: a typed literal.
pub(crate) const LITERAL_TYPES: [&str; 9] = [
    "DATE",
    "TIME",
    "DATETIME",
    "TIMESTAMP",
    "JSON",
    "NUMERIC",
    "DECIMAL",
    "BIGNUMERIC",
    "BIGDECIMAL",
];

/// Reserved keywords that name a function when `(` follows them.
pub(crate) const RESERVED_CALLS: [&str; 8] = [
    "IF", "LEFT", "RIGHT", "GROUPING", "COLLATE", "RANGE", "EXTRACT", "CAST",
];

/// The names `TIMESTAMPADD` and `TIMESTAMPDIFF` go by.
pub(crate) const TIMESTAMP_ADD_NAMES: [&str; 2] = ["TIMESTAMPADD", "DATEADD"];
pub(crate) const TIMESTAMP_DIFF_NAMES: [&str; 3] = ["TIMESTAMPDIFF", "DATEDIFF", "TIMEDIFF"];

/// `unitInMultiUnits`: interval units, singular and plural, each with its shared-tree unit name.
pub(crate) const INTERVAL_UNITS: [(&str, IntervalUnit); 22] = [
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
    ("QUARTER", IntervalUnit::Quarter),
    ("QUARTERS", IntervalUnit::Quarter),
    ("YEAR", IntervalUnit::Year),
    ("YEARS", IntervalUnit::Year),
];

/// The name the tree gives the bucketing, cube and grouping-set calls of `GROUP BY`.
pub(crate) const ROLLUP: &str = "ROLLUP";
pub(crate) const CUBE: &str = "CUBE";
pub(crate) const GROUPING_SETS: &str = "GROUPING SETS";

/// The function name an `IDENTIFIER (expr)` clause becomes, as lineage reads it.
pub(crate) const IDENTIFIER_CLAUSE: &str = "IDENTIFIER";

/// The object words a `CREATE` this parser does not build may name; any other word is refused.
pub(crate) const CREATE_KINDS: [&str; 20] = [
    "AGGREGATE",
    "ASSIGNMENT",
    "BI_CAPACITY",
    "CAPACITY",
    "CONNECTION",
    "DATA_POLICY",
    "EXTERNAL",
    "FUNCTION",
    "INDEX",
    "MODEL",
    "PROCEDURE",
    "PROPERTY",
    "RESERVATION",
    "ROW",
    "SCHEMA",
    "SEARCH",
    "SNAPSHOT",
    "TABLE",
    "VECTOR",
    "PRIVATE",
];

/// `schema_object_kind`'s spellings, longest first where one is a prefix, with the kind each names.
pub(crate) const SCHEMA_OBJECT_KINDS: [(&[&str], &str); 22] = [
    (&["AGGREGATE", "FUNCTION"], "AGGREGATE FUNCTION"),
    (&["APPROX", "VIEW"], "APPROX VIEW"),
    (&["CONNECTION"], "CONNECTION"),
    (&["CONSTANT"], "CONSTANT"),
    (&["DATABASE"], "DATABASE"),
    (
        &["EXTERNAL", "TABLE", "FUNCTION"],
        "EXTERNAL TABLE FUNCTION",
    ),
    (&["EXTERNAL", "TABLE"], "EXTERNAL TABLE"),
    (&["EXTERNAL", "SCHEMA"], "EXTERNAL SCHEMA"),
    (&["FUNCTION"], "FUNCTION"),
    (&["SEARCH", "INDEX"], "SEARCH INDEX"),
    (&["VECTOR", "INDEX"], "VECTOR INDEX"),
    (&["AI", "INDEX"], "AI INDEX"),
    (&["INDEX"], "INDEX"),
    (&["LIVE", "TABLE"], "LIVE TABLE"),
    (&["MATERIALIZED", "VIEW"], "MATERIALIZED VIEW"),
    (&["MODEL"], "MODEL"),
    (&["PROCEDURE"], "PROCEDURE"),
    (&["SCHEMA"], "SCHEMA"),
    (&["SEQUENCE"], "SEQUENCE"),
    (&["VIEW"], "VIEW"),
    (&["PROPERTY", "GRAPH", "TYPE"], "PROPERTY GRAPH TYPE"),
    (&["PROPERTY", "GRAPH"], "PROPERTY GRAPH"),
];

/// The schema object kinds `ALTER` takes; any other is "ALTER <kind> is not supported".
pub(crate) const ALTERABLE_KINDS: [&str; 12] = [
    "APPROX VIEW",
    "CONNECTION",
    "DATABASE",
    "SCHEMA",
    "EXTERNAL SCHEMA",
    "VIEW",
    "MATERIALIZED VIEW",
    "MODEL",
    "SEQUENCE",
    "SEARCH INDEX",
    "VECTOR INDEX",
    "AI INDEX",
];

/// The index kinds, the only kinds that take `ON table`.
pub(crate) const INDEX_KINDS: [&str; 4] = ["INDEX", "SEARCH INDEX", "VECTOR INDEX", "AI INDEX"];

/// The generic entity types BigQuery supports (`generic_entity_type`).
pub(crate) const GENERIC_ENTITY_TYPES: [&str; 6] = [
    "ASSIGNMENT",
    "BI_CAPACITY",
    "CAPACITY",
    "ORGANIZATION",
    "PROJECT",
    "RESERVATION",
];

/// The nested object types BigQuery supports (`generic_sub_entity_type`).
pub(crate) const SUB_ENTITY_TYPES: [&str; 1] = ["REPLICA"];

/// Words that begin an `alter_action`.
pub(crate) const ALTER_ACTION_WORDS: [&str; 7] = [
    "SET", "ADD", "DROP", "ALTER", "RENAME", "REPLACE", "REBUILD",
];

/// The clauses GoogleSQL refuses straight after a `FROM` query (`from_query`).
pub(crate) const AFTER_FROM_QUERY: &[&str] = &[
    "SELECT",
    "WHERE",
    "GROUP",
    "HAVING",
    "QUALIFY",
    "WINDOW",
    "ORDER",
    "LIMIT",
    "UNION",
    "INTERSECT",
    "EXCEPT",
];

/// The reserved words that end a select list, so a comma before one is a trailing comma.
pub(crate) const AFTER_SELECT_LIST: &[&str] = &[
    "FROM",
    "WHERE",
    "GROUP",
    "HAVING",
    "QUALIFY",
    "WINDOW",
    "ORDER",
    "LIMIT",
    "UNION",
    "INTERSECT",
    "EXCEPT",
];

/// GoogleSQL's refusal of a chain mixing set operations without parentheses.
pub(crate) const MIXED_SET_OPERATIONS: &str = "Syntax error: Different set operations cannot be used \
     in the same query without using parentheses for grouping";

/// GoogleSQL's refusal, in analysis, of a chain mixing set operation modifiers.
pub(crate) const MIXED_SET_MODIFIERS: &str = "Different set operation modifiers cannot be used in \
     the same query without using parentheses for grouping";

/// GoogleSQL's refusal of a comma join next to consecutive ON/USING clauses.
pub(crate) const COMMA_AFTER_CONSECUTIVE_CONDITIONS: &str =
    "Syntax error: Comma join is not allowed after consecutive ON/USING clauses";

/// GoogleSQL's refusal of an unparenthesised `RIGHT` or `FULL` join after a comma join.
pub(crate) const OUTER_JOIN_AFTER_COMMA: &str =
    "Syntax error: RIGHT or FULL JOIN must be parenthesized when following a comma join";

/// GoogleSQL's refusal of consecutive ON/USING clauses that do not match the joins needing one.
pub(crate) const JOIN_CONDITION_COUNT: &str = "Syntax error: The number of join conditions does \
     not match the number of joins that require a join condition";

/// The word that begins `TABLE FUNCTION` among the `CREATE` kinds.
pub(crate) const TABLE_WORD: &str = "TABLE";

/// The plain index kind, written without a `SEARCH`, `VECTOR` or `AI` prefix.
pub(crate) const INDEX_KIND: &str = "INDEX";

/// The directions an index key may be ordered in.
pub(crate) const SORT_DIRECTIONS: [&str; 2] = ["ASC", "DESC"];

/// The `CREATE TABLE` source word that makes the source a copy rather than a clone.
pub(crate) const COPY_SOURCE: &str = "COPY";

/// The schema object kind `DROP` refuses, pointing to `DROP FUNCTION` instead.
pub(crate) const AGGREGATE_FUNCTION_KIND: &str = "AGGREGATE FUNCTION";

/// The schema object kind of a materialized view.
pub(crate) const MATERIALIZED_VIEW_KIND: &str = "MATERIALIZED VIEW";

/// The schema object kind of a function, the only kind `DROP` takes parameters for.
pub(crate) const FUNCTION_KIND: &str = "FUNCTION";

/// The schema object kind of a schema, the only kind `CASCADE`, `RESTRICT` and `UNDROP` take.
pub(crate) const SCHEMA_KIND: &str = "SCHEMA";

/// The most paths a `GRANT` or `REVOKE` names: up to two single-word kinds, then the object.
pub(crate) const MAX_GRANT_PATHS: usize = 3;
/// `FOR`: another clause word followed by it begins no clause.
pub(crate) const FOR_WORD: &str = "FOR";
/// The case-insensitive member of the `LIKE` family.
pub(crate) const ILIKE_WORD: &str = "ILIKE";
/// The string concatenation operator.
pub(crate) const CONCAT_OPERATOR: &str = "||";
/// The left shift operator; a right shift is adjacent `>` tokens.
pub(crate) const LEFT_SHIFT_OPERATOR: &str = "<<";
/// The prefix of a system variable such as `@@dataset_id`.
pub(crate) const SYSTEM_VARIABLE_PREFIX: &str = "@@";
/// The boolean literal that is true.
pub(crate) const TRUE_LITERAL: &str = "TRUE";
/// The cast that yields `NULL` instead of failing.
pub(crate) const SAFE_CAST_CALL: &str = "SAFE_CAST";
/// The fewest parameters `(a, b, ...) ->` holds when read as a lambda's parameter list.
pub(crate) const MIN_LAMBDA_LIST_PARAMETERS: usize = 2;
/// The separator between a type's parameters or fields, printed with a space after it.
pub(crate) const TYPE_LIST_SEPARATOR: &str = ",";
/// The fewest `ON` or `USING` clauses after one join that make them consecutive conditions.
pub(crate) const MIN_CONSECUTIVE_CONDITIONS: usize = 2;

/// The pipe `EXTEND` operator, which alone of `EXTEND` and `WINDOW` takes a `WINDOW` clause.
pub(crate) const PIPE_EXTEND: &str = "EXTEND";

/// EXTRACT, whose call must take its keyword reading: GoogleSQL has no ordinary EXTRACT call.
pub(crate) const EXTRACT_CALL: &str = "EXTRACT";

/// The refusal of a statement nested past the parser's depth limit or the tree depth bound.
pub(crate) const TOO_DEEP_MESSAGE: &str = "the statement nests too deeply";

/// The deepest tree a statement may build, flat chains included, so a consumer that recurses over it has a bound.
pub(crate) const MAX_TREE_DEPTH: usize = 8_000;

/// The stack a refused tree's drop takes per level, since the shared tree drops recursively.
pub(crate) const TREE_DROP_FRAME: usize = 1 << 10;
