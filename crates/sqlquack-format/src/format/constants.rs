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
pub(crate) const SELECT_LIST_FAILURE: &str =
    "formatter could not find every select list, whose items without an alias it keeps as written";
/// Why SQL in a dialect the crate was built without, through its feature, is left as written.
pub const NOT_COMPILED_FAILURE: &str =
    "formatter was built without this dialect's feature, so it cannot read its SQL";
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

/// PostgreSQL pattern operators whose quantified operand the guide puts on its own lines.
pub(crate) const PATTERN_OPERATORS: [&str; 2] = ["LIKE", "ILIKE"];

/// The BigQuery keyword of a foreign key, after which a name names the referenced table.
pub(crate) const REFERENCES_KEYWORD: &str = "REFERENCES";
/// The BigQuery keyword whose ON names the indexed table, as in `CREATE SEARCH INDEX ix ON t(c)`.
pub(crate) const INDEX_KEYWORD: &str = "INDEX";
/// The SQL Server keyword that, after CROSS or OUTER, applies a table-valued expression like a join.
pub(crate) const APPLY_KEYWORD: &str = "APPLY";

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
/// Words a FROM continues rather than starting a clause after: `IS DISTINCT FROM` and PostgreSQL's `ROWS FROM (...)`.
pub(crate) const FROM_CONTINUED: [&str; 2] = ["DISTINCT", "ROWS"];
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
pub(crate) const BINARY_OPERATOR_NODES: [(&str, &[&str]); 13] = [
    ("eq", &["="]),
    ("neq", &["!=", "<>"]),
    ("lt", &["<"]),
    ("lte", &["<="]),
    ("gt", &[">"]),
    ("gte", &[">="]),
    ("add", &["+"]),
    ("sub", &["-"]),
    ("mul", &["*"]),
    ("div", &["/"]),
    ("mod", &["%"]),
    ("int_div", &["//"]),
    ("concat", &["||"]),
];
/// The bytes the spacing guard reads as operator characters.
pub(crate) const OPERATOR_BYTES: &str = "+-*/<>=!|&^%~:#?";
/// Operators the spacing guard lexes, longest match first, so a piece is never part of a longer one.
pub(crate) const GUARD_OPERATORS: [&str; 22] = [
    "->>", "<=>", "||", "<=", ">=", "<>", "!=", "=>", "->", "::", ":=", "/*", "*/", "--", "**",
    "//", "<<", ">>", "|>", "!~", "~*", "#>",
];
/// The operator of an assignment the tree keeps outside a binary operator node, such as `SET @v = 1`.
pub(crate) const ASSIGNMENT_OPERATOR: &str = "=";
/// Binary operators that whitespace may separate from a token they touch.
pub(crate) const SEPARABLE_OPERATORS: [&str; 15] = [
    "=", "<>", "!=", "<", ">", "<=", ">=", "+", "-", "*", "/", "//", "%", "||", "->",
];
/// The modulo operator, which DuckDB lexes together with a sign written touching it, as `%-`.
pub(crate) const MODULO_OPERATOR: &str = "%";
/// Tokens a binary operator is never spaced from: name and path accessors.
pub(crate) const UNSEPARABLE_NEIGHBOURS: [&str; 3] = [".", ":", "::"];
/// Keywords after which a clause word is a table alias instead.
pub(crate) const ALIAS_FOLLOWERS: [&str; 3] = ["MATCH_CONDITION", "ON", "USING"];
/// Set operators, each starting its own line between its operands.
pub(crate) const SET_OPERATORS: [&str; 4] = ["UNION", "EXCEPT", "INTERSECT", "MINUS"];
/// Words that continue a set operator, such as `UNION ALL BY NAME` or `UNION ALL STRICT CORRESPONDING BY (a)`.
pub(crate) const SET_OPERATOR_MODIFIERS: [&str; 7] = [
    "ALL",
    "DISTINCT",
    "BY",
    "NAME",
    "CORRESPONDING",
    "STRICT",
    "ON",
];
/// The kinds of a BigQuery value-table SELECT, such as `SELECT AS STRUCT`.
pub(crate) const VALUE_TABLE_KINDS: [&str; 2] = ["STRUCT", "VALUE"];
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
