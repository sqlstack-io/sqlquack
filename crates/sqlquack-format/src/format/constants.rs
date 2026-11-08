//! Constants of format.

/// Spaces per indent step when the caller does not choose.
pub const DEFAULT_INDENT: usize = 2;

pub(crate) const TOKENIZE_FAILURE: &str = "formatter tokenization failed";
pub(crate) const PARSE_FAILURE: &str = "formatter could not parse SQL";
pub(crate) const NOT_IDEMPOTENT_FAILURE: &str = "formatter output is not idempotent";
pub(crate) const TOKEN_PRESERVATION_FAILURE: &str =
    "formatter would change authored SQL tokens, not only layout";
pub(crate) const COMMENT_ATTACHMENT_FAILURE: &str =
    "formatter could not preserve comment token attachments";
pub(crate) const STRING_BREAK_FAILURE: &str =
    "formatter would join adjacent string literals across a line break";
pub(crate) const ALIGNMENT_FAILURE: &str =
    "formatter could not align its layout with authored tokens";
pub(crate) const SEPARATION_FAILURE: &str =
    "formatter would separate tokens written touching where the space could change their meaning";
pub(crate) const JOIN_FAILURE: &str =
    "formatter would join tokens written apart where they could read as one";
pub(crate) const SQLSERVER_OPAQUE_STATEMENT_FAILURE: &str =
    "formatter does not lay out a statement the SQL Server parser keeps as opaque text";
pub(crate) const BATCH_SEPARATOR_FAILURE: &str = "formatter does not lay out a GO batch separator, which a client reads and SQL Server never sees";
/// The word a SQL Server client reads, alone on its line, as the end of a batch.
pub(crate) const BATCH_SEPARATOR: &str = "GO";
/// The start of a comment that runs to the end of its line.
pub(crate) const LINE_COMMENT_START: &str = "--";
/// The start of a BigQuery comment that runs to the end of its line.
pub(crate) const HASH_COMMENT_START: char = '#';
/// The start of a `/* ... */` comment.
pub(crate) const BLOCK_COMMENT_START: &str = "/*";
/// The end of a `/* ... */` comment.
pub(crate) const BLOCK_COMMENT_END: &str = "*/";
pub(crate) const SELECT_LIST_FAILURE: &str =
    "formatter could not find every select list, whose items without an alias it keeps as written";
pub(crate) const UNSUPPORTED_DIALECT_FAILURE: &str = "formatter does not support this dialect yet; only Snowflake, DuckDB and SQL Server have a format gate";
/// The dialects whose format corpus subset gates the formatter.
pub(crate) const FORMAT_GATED_DIALECTS: [crate::format::models::FormatDialect; 3] = [
    crate::format::models::FormatDialect::Snowflake,
    crate::format::models::FormatDialect::DuckDB,
    crate::format::models::FormatDialect::SqlServer,
];
pub(crate) const DEPTH_FAILURE: &str =
    "formatter does not lay out SQL whose parse tree is more than 512 levels deep";
/// The deepest parse tree the formatter walks; a deeper one, such as a chain of thousands of `+`, is refused before it can exhaust the stack.
pub(crate) const MAX_TREE_DEPTH: usize = 512;
pub(crate) const TYPE_DEPTH_FAILURE: &str =
    "formatter does not lay out data types nested more than 64 levels deep";
/// The deepest data type the formatter lets the parser build: a run of bracket suffixes such as `INT[][]`, nested type constructors, or both alternating.
pub(crate) const MAX_TYPE_NESTING: usize = 64;
/// The types whose parameters are written in angle brackets, such as `ARRAY<INT>`.
pub(crate) const PARAMETERISED_TYPES: [&str; 5] = ["ARRAY", "STRUCT", "MAP", "OBJECT", "RANGE"];
/// The type names whose parameters are named fields, such as `STRUCT(a INT)`.
pub(crate) const STRUCT_TYPE_WORDS: [&str; 5] = ["STRUCT", "ROW", "OBJECT", "UNION", "RECORD"];
/// The type names whose parameters are data types in angle brackets or parentheses, such as `ARRAY<INT>` or `STRUCT(a INT)`.
pub(crate) const TYPE_CONSTRUCTOR_WORDS: [&str; 10] = [
    "ARRAY", "STRUCT", "MAP", "RANGE", "OBJECT", "ROW", "RECORD", "UNION", "VECTOR", "NULLABLE",
];
pub(crate) const UNSUPPORTED_SYNTAX_FAILURE: &str =
    "formatter does not lay out bind variables or string stage references with options";

/// Snowflake calls the guide keeps on one line however wide; others split past the guide width.
pub(crate) const SNOWFLAKE_INLINE_CALLS: [&str; 81] = [
    "ABS",
    "AI_AGG",
    "AI_SUMMARIZE_AGG",
    "APPROX_PERCENTILE_ACCUMULATE",
    "APPROX_PERCENTILE_COMBINE",
    "APPROX_PERCENTILE_ESTIMATE",
    "APPROX_TOP_K",
    "APPROX_TOP_K_ACCUMULATE",
    "APPROX_TOP_K_COMBINE",
    "APPROX_TOP_K_ESTIMATE",
    "ARRAY_APPEND",
    "ARRAY_CONTAINS",
    "ARRAY_DISTINCT",
    "ARRAY_PREPEND",
    "ARRAY_REMOVE",
    "ASIN",
    "ATAN",
    "ATAN2",
    "BITMAP_CONSTRUCT_AGG",
    "BIT_AND",
    "BIT_OR",
    "BIT_XOR",
    "BOOLXOR_AGG",
    "CAST",
    "CEIL",
    "CEILING",
    "COALESCE",
    "COS",
    "COUNT",
    "CURRENT_SCHEMAS",
    "DAY",
    "DAYOFMONTH",
    "DAYOFWEEK",
    "DAYOFYEAR",
    "DEGREES",
    "EXP",
    "FLATTEN",
    "FLOOR",
    "GENERATOR",
    "HASH_AGG",
    "HLL",
    "HOUR",
    "KURTOSIS",
    "LOG",
    "MAP_CONTAINS_KEY",
    "MAP_KEYS",
    "MAX",
    "MIN",
    "MINUTE",
    "MODE",
    "MONTH",
    "MONTHS_BETWEEN",
    "NEXT_DAY",
    "NTH_VALUE",
    "OBJECT_AGG",
    "PARSE_JSON",
    "PERCENTILE_CONT",
    "PERCENTILE_DISC",
    "POW",
    "QUARTER",
    "RADIANS",
    "RANDOM",
    "REGR_AVGX",
    "REGR_AVGY",
    "REGR_COUNT",
    "REGR_INTERCEPT",
    "REGR_R2",
    "REGR_SLOPE",
    "REGR_SXX",
    "REGR_SXY",
    "REGR_SYY",
    "SECOND",
    "STDDEV",
    "STDDEV_POP",
    "STDDEV_SAMP",
    "SUM",
    "TABLE",
    "TAN",
    "TRIM",
    "TRY_CAST",
    "YEAR",
];
/// DuckDB calls the guide keeps on one line however wide.
pub(crate) const DUCKDB_INLINE_CALLS: [&str; 25] = [
    "AVG",
    "CAST",
    "COALESCE",
    "COUNT",
    "CUME_DIST",
    "EPOCH_MS",
    "EXP",
    "FIRST",
    "JSON_EXTRACT",
    "LAST_VALUE",
    "LEN",
    "LENGTH",
    "LOG",
    "LOGICAL_OR",
    "MIN",
    "MODE",
    "NTH_VALUE",
    "NTILE",
    "PERCENTILE_CONT",
    "PERCENTILE_DISC",
    "PERCENT_RANK",
    "RANK",
    "SUM",
    "TRIM",
    "TRY_CAST",
];
/// PostgreSQL calls the guide keeps on one line however wide.
pub(crate) const POSTGRES_INLINE_CALLS: [&str; 14] = [
    "ARRAY_LENGTH",
    "CAST",
    "COUNT",
    "DATE_PART",
    "EXP",
    "EXTRACT",
    "JSON_AGG",
    "LOGICAL_OR",
    "MODE",
    "PERCENTILE_CONT",
    "SUM",
    "TRIM",
    "TRY_CAST",
    "XMLELEMENT",
];

/// SQL Server calls the guide keeps on one line however wide.
pub(crate) const SQLSERVER_INLINE_CALLS: [&str; 2] = ["CAST", "SUBSTRING"];
/// The SQL Server keyword that, after CROSS or OUTER, applies a table-valued expression like a join.
pub(crate) const SQLSERVER_APPLY_KEYWORD: &str = "APPLY";
/// The words before APPLY in a SQL Server apply.
pub(crate) const SQLSERVER_APPLY_PREFIXES: [&str; 2] = ["CROSS", "OUTER"];
/// SQL Server keywords after which a name names a table, view, routine, index or other object, kept as written since the database collation may be case-sensitive.
pub(crate) const SQLSERVER_OBJECT_PRECEDERS: [&str; 22] = [
    "APPLY",
    "DELETE",
    "EXEC",
    "EXECUTE",
    "FROM",
    "FUNCTION",
    "INDEX",
    "INSERT",
    "INTO",
    "JOIN",
    "MERGE",
    "ON",
    "PROC",
    "PROCEDURE",
    "REFERENCES",
    "SEQUENCE",
    "STATISTICS",
    "TABLE",
    "TRIGGER",
    "UPDATE",
    "USING",
    "VIEW",
];
/// SQL Server clauses that hold a comma-separated list, which bound the search for the FROM a comma belongs to.
pub(crate) const SQLSERVER_LIST_CLAUSES: [&str; 9] = [
    "BY", "FROM", "OUTPUT", "RETURNS", "SELECT", "SET", "VALUES", "WHERE", "HAVING",
];
/// SQL Server keywords the guide separates from a following `(`: `TOP (n)` and a `WITH (hint)` table hint.
pub(crate) const SQLSERVER_SPACED_BEFORE_PARENTHESIS: [&str; 2] = ["TOP", "WITH"];

/// The line width past which the layout guide splits a call's arguments or a CASE.
pub(crate) const GUIDE_WIDTH: usize = 80;
/// Spaces per indent step of the layout guide; printing rescales it to the requested indent.
pub(crate) const GUIDE_INDENT: usize = 2;
/// The measured width of a literal, path key or type name before its sequence number's digits.
pub(crate) const PLACEHOLDER_WIDTH: usize = 20;
/// The added width of a `::` cast, which the guide measures as `CAST(value AS type)`.
pub(crate) const POSTFIX_CAST_WIDTH: usize = 10;
/// Keywords the guide always separates from a following `(`.
pub(crate) const SPACED_BEFORE_PARENTHESIS: [&str; 44] = [
    "ALL",
    "AND",
    "ANY",
    "AS",
    "AT",
    "BEFORE",
    "BERNOULLI",
    "BLOCK",
    "BY",
    "CHANGES",
    "CUBE",
    "DISTINCT",
    "ELSE",
    "END",
    "FROM",
    "GROUP",
    "HAVING",
    "IN",
    "JOIN",
    "KEY",
    "LATERAL",
    "MATCH_CONDITION",
    "NOT",
    "NULL",
    "ON",
    "OR",
    "OVER",
    "PERCENT",
    "QUALIFY",
    "REPEATABLE",
    "RESERVOIR",
    "ROLLUP",
    "SAMPLE",
    "SEED",
    "SELECT",
    "SETS",
    "SOME",
    "SYSTEM",
    "TABLESAMPLE",
    "THEN",
    "USING",
    "VALUES",
    "WHEN",
    "WHERE",
];
/// Words that may precede JOIN in a join keyword.
pub(crate) const JOIN_PREFIXES: [&str; 12] = [
    "NATURAL",
    "ASOF",
    "LEFT",
    "RIGHT",
    "FULL",
    "INNER",
    "OUTER",
    "CROSS",
    "SEMI",
    "ANTI",
    "POSITIONAL",
    "DIRECTED",
];
/// Typed literal keywords whose string the guide measures with it as one placeholder.
pub(crate) const TYPED_LITERALS: [&str; 7] = [
    "DATE",
    "TIME",
    "TIMESTAMP",
    "TIMESTAMPTZ",
    "TIMESTAMP_NTZ",
    "TIMESTAMP_LTZ",
    "TIMESTAMP_TZ",
];
/// Words that continue a `::` cast type, such as `DOUBLE PRECISION`.
pub(crate) const TYPE_CONTINUATIONS: [&str; 6] =
    ["PRECISION", "VARYING", "WITH", "WITHOUT", "TIME", "ZONE"];
/// Calls that always put each argument on its own line.
pub(crate) const GROUPING_CALLS: [&str; 3] = ["ROLLUP", "CUBE", "SETS"];
/// Clause words of a Snowflake SEMANTIC_VIEW call, each starting its own line.
pub(crate) const SEMANTIC_VIEW_CLAUSES: [&str; 4] = ["METRICS", "DIMENSIONS", "FACTS", "WHERE"];
/// The start of a placeholder name, such as SQLBuild's `__ref(...)`, which keeps its case when called.
pub(crate) const PLACEHOLDER_NAME_PREFIX: &str = "__";
/// Words after which a name with `(` is defined, not called: a table's column list or a macro's parameters.
pub(crate) const CALL_TARGET_KEYWORDS: [&str; 3] = ["INTO", "TABLE", "MACRO"];
/// Words after a CTE's `AS` that start its body.
pub(crate) const CTE_BODY_PREFIXES: [&str; 2] = ["MATERIALIZED", "NOT"];
/// The keyword that starts an INSERT statement.
pub(crate) const INSERT_KEYWORD: &str = "INSERT";
/// The keyword of a VALUES list.
pub(crate) const VALUES_KEYWORD: &str = "VALUES";
/// Words that start the rows an INSERT adds, each from a new line.
pub(crate) const INSERT_BODIES: [&str; 3] = ["VALUES", "SELECT", "WITH"];
/// Clause keywords that start a line of a SELECT.
pub(crate) const CLAUSE_KEYWORDS: [&str; 8] = [
    "FROM", "WHERE", "HAVING", "QUALIFY", "WINDOW", "LIMIT", "OFFSET", "FETCH",
];
/// Keywords that whitespace may separate from a token they touch, such as `)FROM`.
pub(crate) const SEPARABLE_KEYWORDS: [&str; 14] = [
    "SELECT",
    "FROM",
    "WHERE",
    "GROUP",
    "HAVING",
    "QUALIFY",
    "WINDOW",
    "ORDER",
    "LIMIT",
    "OFFSET",
    "FETCH",
    "UNION",
    "EXCEPT",
    "INTERSECT",
];
/// Binary operator nodes of the parse tree, with the spellings of their operator token.
pub(crate) const BINARY_OPERATOR_NODES: [(&str, &[&str]); 10] = [
    ("eq", &["="]),
    ("neq", &["!=", "<>"]),
    ("lt", &["<"]),
    ("lte", &["<="]),
    ("gt", &[">"]),
    ("gte", &[">="]),
    ("add", &["+"]),
    ("sub", &["-"]),
    ("mul", &["*"]),
    ("concat", &["||"]),
];
/// The bytes the spacing guard reads as operator characters.
pub(crate) const OPERATOR_BYTES: &str = "+-*/<>=!|&^%~:#?";
/// The bytes DuckDB lexes as operator characters beyond the shared ones.
pub(crate) const DUCKDB_OPERATOR_BYTES: &str = "@";
/// The bytes PostgreSQL lexes as operator characters beyond the shared ones.
pub(crate) const POSTGRES_OPERATOR_BYTES: &str = "@`";
/// Operators the spacing guard lexes, longest match first, so a piece is never part of a longer one.
pub(crate) const GUARD_OPERATORS: [&str; 22] = [
    "->>", "<=>", "||", "<=", ">=", "<>", "!=", "=>", "->", "::", ":=", "/*", "*/", "--", "**",
    "//", "<<", ">>", "|>", "!~", "~*", "#>",
];
/// Binary operators that whitespace may separate from a token they touch.
pub(crate) const SEPARABLE_OPERATORS: [&str; 12] = [
    "=", "<>", "!=", "<", ">", "<=", ">=", "+", "-", "*", "||", "->",
];
/// Tokens a binary operator is never spaced from: name and path accessors.
pub(crate) const UNSEPARABLE_NEIGHBOURS: [&str; 3] = [".", ":", "::"];
/// Keywords after which a clause word is a table alias instead.
pub(crate) const ALIAS_FOLLOWERS: [&str; 3] = ["MATCH_CONDITION", "ON", "USING"];
/// Set operators, each starting its own line between its operands.
pub(crate) const SET_OPERATORS: [&str; 4] = ["UNION", "EXCEPT", "INTERSECT", "MINUS"];
/// Words that continue a set operator, such as `UNION ALL BY NAME`.
pub(crate) const SET_OPERATOR_MODIFIERS: [&str; 5] =
    ["ALL", "DISTINCT", "BY", "NAME", "CORRESPONDING"];
/// The branch keywords of a CASE expression.
pub(crate) const CASE_BRANCHES: [&str; 3] = ["WHEN", "THEN", "ELSE"];
/// The sampling unit spaced before `(` after SAMPLE or TABLESAMPLE.
pub(crate) const SAMPLE_ROW_KEYWORD: &str = "ROW";
/// The Snowflake call whose clauses start their own lines.
pub(crate) const SEMANTIC_VIEW_CALL: &str = "SEMANTIC_VIEW";
/// An opening parenthesis.
pub(crate) const OPEN_PARENTHESIS: &str = "(";
/// A closing parenthesis.
pub(crate) const CLOSE_PARENTHESIS: &str = ")";
/// DuckDB's call that expands to columns, which a `*` before it unpacks.
pub(crate) const COLUMNS_CALL: &str = "COLUMNS";
/// DuckDB's lambda keyword, whose parameters end at a `:`.
pub(crate) const LAMBDA_KEYWORD: &str = "LAMBDA";
/// Keywords whose count a DuckDB `%` may follow as a percentage.
pub(crate) const SAMPLE_KEYWORDS: [&str; 3] = ["SAMPLE", "TABLESAMPLE", "LIMIT"];
/// Keywords after which a `(` opens a parenthesised expression, not an argument list.
pub(crate) const EXPRESSION_LEADERS: [&str; 11] = [
    "AND", "OR", "NOT", "WHERE", "ON", "HAVING", "QUALIFY", "WHEN", "THEN", "ELSE", "SELECT",
];
