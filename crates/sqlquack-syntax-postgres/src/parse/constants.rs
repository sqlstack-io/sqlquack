//! Operator precedence levels from gram.y's declarations, lowest first, and keyword sets.

/// `%left OR`.
pub(crate) const OR: u8 = 1;
/// `%left AND`.
pub(crate) const AND: u8 = 2;
/// `%right NOT`.
pub(crate) const NOT: u8 = 3;
/// `%nonassoc IS ISNULL NOTNULL`.
pub(crate) const IS: u8 = 4;
/// `%nonassoc '<' '>' '=' LESS_EQUALS GREATER_EQUALS NOT_EQUALS`.
pub(crate) const COMPARE: u8 = 5;
/// `%nonassoc BETWEEN IN_P LIKE ILIKE SIMILAR NOT_LA`.
pub(crate) const LIKE: u8 = 6;
/// `%left Op OPERATOR`.
pub(crate) const OPERATOR: u8 = 8;
/// `%left '+' '-'`.
pub(crate) const ADD: u8 = 9;
/// `%left '*' '/' '%'`.
pub(crate) const MULTIPLY: u8 = 10;
/// `%left '^'`.
pub(crate) const EXPONENT: u8 = 11;
/// `%left AT`.
pub(crate) const AT: u8 = 12;
/// `%left COLLATE`.
pub(crate) const COLLATE: u8 = 13;
/// `%right UMINUS`.
pub(crate) const UNARY: u8 = 14;
/// `%left TYPECAST`.
pub(crate) const TYPECAST: u8 = 16;

/// Reserved keywords that still begin an expression.
pub(crate) const EXPRESSION_KEYWORDS: [&str; 22] = [
    "array",
    "case",
    "cast",
    "current_catalog",
    "current_date",
    "current_role",
    "current_schema",
    "current_time",
    "current_timestamp",
    "current_user",
    "default",
    "false",
    "localtime",
    "localtimestamp",
    "not",
    "null",
    "session_user",
    "system_user",
    "true",
    "unique",
    "user",
    "operator",
];

/// SQL/JSON special calls (gram.y's `func_expr_common_subexpr`), checked and kept as gaps.
pub(crate) const JSON_CALLS: [&str; 11] = [
    "json",
    "json_array",
    "json_arrayagg",
    "json_exists",
    "json_object",
    "json_objectagg",
    "json_query",
    "json_scalar",
    "json_serialize",
    "json_value",
    "merge_action",
];

/// The SQL/JSON aggregates, which take `FILTER` and `OVER` after their arguments.
pub(crate) const JSON_AGGREGATES: [&str; 2] = ["json_arrayagg", "json_objectagg"];

/// SQL/XML special calls and `TREAT`, checked and kept as gaps.
pub(crate) const XML_CALLS: [&str; 9] = [
    "treat",
    "xmlconcat",
    "xmlelement",
    "xmlexists",
    "xmlforest",
    "xmlparse",
    "xmlpi",
    "xmlroot",
    "xmlserialize",
];

/// The SQL-value functions written without parentheses, and whether each takes a precision.
pub(crate) const VALUE_FUNCTIONS: [(&str, bool); 12] = [
    ("current_catalog", false),
    ("current_date", false),
    ("current_role", false),
    ("current_schema", false),
    ("current_time", true),
    ("current_timestamp", true),
    ("current_user", false),
    ("localtime", true),
    ("localtimestamp", true),
    ("session_user", false),
    ("system_user", false),
    ("user", false),
];

/// Words that start a query: a parenthesis holding one is a subquery.
pub(crate) const QUERY_STARTS: [&str; 4] = ["select", "values", "with", "table"];

/// Words that begin a statement in gram.y's `stmt`; any other first word is a syntax error.
pub(crate) const STATEMENT_STARTS: [&str; 46] = [
    "abort",
    "alter",
    "analyze",
    "analyse",
    "begin",
    "call",
    "checkpoint",
    "close",
    "cluster",
    "comment",
    "commit",
    "copy",
    "create",
    "deallocate",
    "declare",
    "delete",
    "discard",
    "do",
    "drop",
    "end",
    "execute",
    "explain",
    "fetch",
    "grant",
    "import",
    "insert",
    "listen",
    "load",
    "lock",
    "merge",
    "move",
    "notify",
    "prepare",
    "reassign",
    "refresh",
    "reindex",
    "release",
    "reset",
    "revoke",
    "rollback",
    "savepoint",
    "security",
    "set",
    "show",
    "start",
    "truncate",
];

/// The rest of gram.y's statement starts.
pub(crate) const STATEMENT_STARTS_MORE: [&str; 3] = ["unlisten", "vacuum", "update"];

/// Keywords that begin a `ConstTypename` or `ConstInterval`, and so a typed literal.
pub(crate) const TYPE_KEYWORDS: [&str; 20] = [
    "bigint",
    "bit",
    "boolean",
    "char",
    "character",
    "dec",
    "decimal",
    "double",
    "float",
    "int",
    "integer",
    "interval",
    "json",
    "national",
    "nchar",
    "numeric",
    "real",
    "smallint",
    "time",
    "timestamp",
];

/// The prefix operator the shared tree holds as bitwise not.
pub(crate) const BITWISE_NOT: &str = "~";
/// The case-insensitive member of the `LIKE` family.
pub(crate) const ILIKE: &str = "ilike";
/// The `INTERVAL` field that takes a fractional-seconds precision.
pub(crate) const SECOND_FIELD: &str = "second";
/// `FLOAT(p)` with at most this many bits is `real`; more is `double precision`.
pub(crate) const FLOAT4_MAX_PRECISION: u32 = 24;
/// `FLOAT(p)` takes at most this many bits; gram.y refuses more, and zero, with 22023.
pub(crate) const FLOAT8_MAX_PRECISION: u32 = 53;
/// The schema of the built-in types, which a qualified type name may name.
pub(crate) const CATALOGUE_SCHEMA: &str = "pg_catalog";
/// `numeric`, the one built-in generic type name taking two modifiers.
pub(crate) const NUMERIC_TYPE: &str = "numeric";
/// The most fractional-second digits a time, timestamp or interval keeps.
pub(crate) const MAX_SECOND_PRECISION: u32 = 6;
/// An `any_name` has at most this many dotted parts: catalog, schema and name.
pub(crate) const MAX_QUALIFIED_NAME_PARTS: usize = 3;
/// `implicit_row`: a parenthesized list is a row only with at least this many items.
pub(crate) const MIN_IMPLICIT_ROW_ITEMS: usize = 2;

/// The function name the tree gives a `ROW (...)` constructor.
pub(crate) const ROW_CONSTRUCTOR: &str = "ROW";

/// The encodings `FORMAT JSON ENCODING` accepts, compared without case.
pub(crate) const JSON_ENCODINGS: [&str; 3] = ["utf8", "utf16", "utf32"];

/// Fields an `INTERVAL` may name, and those that may follow `TO`.
pub(crate) const INTERVAL_FIELDS: [&str; 6] = ["year", "month", "day", "hour", "minute", "second"];

/// The function `expr AT LOCAL` calls, as PostgreSQL's raw tree names it with one argument.
pub(crate) const TIMEZONE_FUNCTION: &str = "timezone";

/// The right side of `IS UNKNOWN`, which PostgreSQL keeps apart from `IS NULL`.
pub(crate) const UNKNOWN_TEST: &str = "UNKNOWN";

/// The right side of `IS NOT UNKNOWN`.
pub(crate) const NOT_UNKNOWN_TEST: &str = "NOT UNKNOWN";

/// The catalogue name of `BIT`.
pub(crate) const BIT_TYPE: &str = "bit";

/// The catalogue name of `BIT VARYING`.
pub(crate) const VARBIT_TYPE: &str = "varbit";

/// The catalogue name of `INTERVAL`.
pub(crate) const INTERVAL_TYPE: &str = "interval";
