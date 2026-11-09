//! The layout's indentation, width and clause words.

/// One level of indentation.
pub const INDENT: &str = "    ";

/// The longest query or `CASE` kept on one line, counted in characters other than whitespace.
pub const INLINE_WIDTH: usize = 60;

/// Words that start a clause of a query on a new line.
pub const CLAUSE_WORDS: [&str; 8] = [
    "SELECT", "FROM", "WHERE", "HAVING", "QUALIFY", "WINDOW", "LIMIT", "OFFSET",
];

/// Words that open a query inside parentheses, besides a set operation at its level.
pub const QUERY_OPENERS: [&str; 3] = ["SELECT", "WITH", "VALUES"];

/// Words that start a clause when `BY` follows them.
pub const BY_CLAUSE_WORDS: [&str; 2] = ["GROUP", "ORDER"];

/// Set operations, which stand on a line of their own.
pub const SET_OPERATIONS: [&str; 3] = ["UNION", "INTERSECT", "EXCEPT"];

/// Words that can come between a set operation and the query after it.
pub const SET_OPERATION_MODIFIERS: [&str; 4] = ["ALL", "DISTINCT", "BY", "NAME"];

/// Words that can come before `JOIN` in a join.
pub const JOIN_MODIFIERS: [&str; 11] = [
    "LEFT",
    "RIGHT",
    "FULL",
    "INNER",
    "OUTER",
    "CROSS",
    "NATURAL",
    "ASOF",
    "POSITIONAL",
    "SEMI",
    "ANTI",
];

/// Words after which a parenthesis groups an expression rather than calling a function.
pub const NON_CALL_WORDS: [&str; 15] = [
    "SELECT", "WHERE", "AND", "OR", "NOT", "WHEN", "THEN", "ELSE", "IN", "ON", "AS", "HAVING",
    "QUALIFY", "BY", "IS",
];

/// Words that can come between `SELECT` and its first item.
pub const SELECT_MODIFIERS: [&str; 3] = ["DISTINCT", "ALL", "ON"];
