//! The grammar's fixed words, limits and messages (`parse.y`, `sqliteLimit.h`).

/// Recursion and chained operators past this fail as a limit, near `SQLITE_MAX_EXPR_DEPTH`.
pub(crate) const MAX_DEPTH: usize = 1000;

/// The deepest tree a statement may build, the cap the shared tree's stack-safe walks are tested to.
pub(crate) const MAX_TREE_DEPTH: usize = 8000;

/// Stack the recursion keeps free before it grows its stack, and how much it adds each time.
pub(crate) const STACK_RED_ZONE: usize = 256 * 1024;
pub(crate) const STACK_SEGMENT: usize = 4 * 1024 * 1024;

/// The most `.`-separated parts a column reference has: schema, table and column.
pub(crate) const MAX_COLUMN_PARTS: usize = 3;

/// The most numbers a type name takes in parentheses.
pub(crate) const MAX_TYPE_MODIFIERS: usize = 2;

/// The most join words before `JOIN`.
pub(crate) const MAX_JOIN_WORDS: usize = 3;

/// The message past the depth limit.
pub(crate) const TOO_DEEP: &str = "the statement nests too deeply";

/// The message for a statement that ends too soon.
pub(crate) const INCOMPLETE_INPUT: &str = "incomplete input";

/// The keywords that start a statement other than a query, each kept whole as a gap named for it.
pub(crate) const STATEMENT_KEYWORDS: [&str; 20] = [
    "EXPLAIN",
    "BEGIN",
    "COMMIT",
    "END",
    "ROLLBACK",
    "SAVEPOINT",
    "RELEASE",
    "CREATE",
    "DROP",
    "INSERT",
    "REPLACE",
    "UPDATE",
    "DELETE",
    "PRAGMA",
    "VACUUM",
    "ATTACH",
    "DETACH",
    "REINDEX",
    "ANALYZE",
    "ALTER",
];

/// The statements a `WITH` clause may introduce besides a query.
pub(crate) const WITH_STATEMENTS: [&str; 4] = ["INSERT", "REPLACE", "UPDATE", "DELETE"];

/// `SQLITE_MAX_COMPOUND_SELECT`: the most terms a compound query may have.
pub(crate) const MAX_COMPOUND_SELECT: usize = 500;

/// `SQLITE_MAX_FUNCTION_ARG`: the most arguments a call may have.
pub(crate) const MAX_FUNCTION_ARG: usize = 1000;

/// `SQLITE_MAX_COLUMN`: the most terms an aggregate's `ORDER BY` may have.
pub(crate) const MAX_COLUMN: usize = 2000;

/// The join keywords `sqlite3JoinType` reads, with the join-type bits each sets.
pub(crate) const JOIN_WORDS: [(&str, u8); 7] = [
    ("NATURAL", JT_NATURAL),
    ("LEFT", JT_LEFT | JT_OUTER),
    ("OUTER", JT_OUTER),
    ("RIGHT", JT_RIGHT | JT_OUTER),
    ("FULL", JT_LEFT | JT_RIGHT | JT_OUTER),
    ("INNER", JT_INNER),
    ("CROSS", JT_INNER | JT_CROSS),
];

pub(crate) const JT_INNER: u8 = 0x01;
pub(crate) const JT_CROSS: u8 = 0x02;
pub(crate) const JT_NATURAL: u8 = 0x04;
pub(crate) const JT_LEFT: u8 = 0x08;
pub(crate) const JT_RIGHT: u8 = 0x10;
pub(crate) const JT_OUTER: u8 = 0x20;

/// The grammar actions' messages.
pub(crate) const COMPOUND_TERMS: &str = "too many terms in compound SELECT";
pub(crate) const UNSUPPORTED_FRAME: &str = "unsupported frame specification";
pub(crate) const DISTINCT_WINDOW: &str = "DISTINCT is not supported for window functions";
pub(crate) const ORDER_BY_TERMS: &str = "too many terms in ORDER BY clause";

/// `SQLITE_MAX_VARIABLE_NUMBER`: the highest `?NNN`.
pub(crate) const MAX_VARIABLE_NUMBER: u32 = 32766;
/// The message for a `?NNN` out of range.
pub(crate) const VARIABLE_RANGE: &str = "variable number must be between ?1 and ?32766";
