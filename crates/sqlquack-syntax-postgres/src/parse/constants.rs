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

/// Keywords whose special call syntax (gram.y's `func_expr_common_subexpr`) is gap-only here.
pub(crate) const GAP_CALLS: [&str; 19] = [
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
    "treat",
    "xmlconcat",
    "xmlelement",
    "xmlexists",
    "xmlforest",
    "xmlparse",
    "xmlpi",
    "xmlroot",
];

/// `XMLSERIALIZE`, also gap-only; kept apart so `GAP_CALLS` stays sorted by family.
pub(crate) const GAP_CALL_XMLSERIALIZE: &str = "xmlserialize";

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

/// Fields an `INTERVAL` may name, and those that may follow `TO`.
pub(crate) const INTERVAL_FIELDS: [&str; 6] = ["year", "month", "day", "hour", "minute", "second"];
