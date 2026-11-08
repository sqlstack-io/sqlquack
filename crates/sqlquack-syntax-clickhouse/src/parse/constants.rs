//! Keyword lists and limits of ClickHouse's parsers.

use sqlquack_syntax::expressions::types::models::IntervalUnit;

/// Nesting deeper than this is refused as a parser check (as `max_parser_depth`), not syntax.
pub(crate) const MAX_DEPTH: usize = 200;

/// The deepest statement tree admitted, the shared parser's AST depth guard (`max_ast_depth`).
pub(crate) const MAX_TREE_DEPTH: usize = 512;

/// The most parts a table name takes, `database.table`, as `parseDatabaseAndTableAsAST` reads it.
pub(crate) const MAX_TABLE_NAME_PARTS: usize = 2;

/// Stack left below which a deeper level first moves to a new stack segment.
pub(crate) const STACK_RED_ZONE: usize = match cfg!(debug_assertions) {
    true => 4 * 1024 * 1024,
    false => 1024 * 1024,
};

/// The size of each new stack segment.
pub(crate) const STACK_SEGMENT: usize = 8 * 1024 * 1024;

/// `ParserAlias::restricted_keywords`: words that are never an alias written without `AS`.
pub(crate) const ALIAS_RESTRICTED: [&str; 42] = [
    "ALL",
    "ANTI",
    "ANY",
    "ARRAY",
    "ASOF",
    "BETWEEN",
    "CROSS",
    "PASTE",
    "FINAL",
    "FORMAT",
    "FROM",
    "FULL",
    "GLOBAL",
    "GROUP",
    "HAVING",
    "ILIKE",
    "INNER",
    "INTO",
    "JOIN",
    "LEFT",
    "LIKE",
    "LIMIT",
    "NOT",
    "OFFSET",
    "ON",
    "ONLY",
    "ORDER",
    "PARALLEL",
    "PREWHERE",
    "RIGHT",
    "SAMPLE",
    "SEMI",
    "SETTINGS",
    "UNION",
    "USING",
    "WHERE",
    "WINDOW",
    "QUALIFY",
    "WITH",
    "INTERSECT",
    "EXCEPT",
    "ELSE",
];

/// Operator priorities of `ParserExpressionImpl::operators_table`; a higher one binds tighter.
pub(crate) const PRIORITY_LAMBDA: u8 = 1;
pub(crate) const PRIORITY_TERNARY: u8 = 2;
/// The `:` of `cond ? a : b`, which ends its middle operand.
pub(crate) const PRIORITY_TERNARY_ELSE: u8 = 3;
pub(crate) const PRIORITY_OR: u8 = 3;
pub(crate) const PRIORITY_AND: u8 = 4;
pub(crate) const PRIORITY_NOT: u8 = 5;
pub(crate) const PRIORITY_IS: u8 = 6;
pub(crate) const PRIORITY_BETWEEN: u8 = 7;
/// The `AND` of `BETWEEN low AND high` (`finish_between_operator`).
pub(crate) const PRIORITY_FINISH_BETWEEN: u8 = 8;
pub(crate) const PRIORITY_COMPARE: u8 = 9;
pub(crate) const PRIORITY_CONCAT: u8 = 10;
pub(crate) const PRIORITY_ADD: u8 = 11;
pub(crate) const PRIORITY_MULTIPLY: u8 = 12;
pub(crate) const PRIORITY_NEGATE: u8 = 13;
pub(crate) const PRIORITY_ACCESS: u8 = 14;

/// Interval kinds `parseIntervalKind` reads, each with its spellings.
pub(crate) const INTERVAL_UNITS: [(IntervalUnit, &[&str]); 11] = [
    (
        IntervalUnit::Nanosecond,
        &["NANOSECOND", "NANOSECONDS", "SQL_TSI_NANOSECOND", "NS"],
    ),
    (
        IntervalUnit::Microsecond,
        &["MICROSECOND", "MICROSECONDS", "SQL_TSI_MICROSECOND"],
    ),
    (
        IntervalUnit::Millisecond,
        &["MILLISECOND", "MILLISECONDS", "SQL_TSI_MILLISECOND", "MS"],
    ),
    (
        IntervalUnit::Second,
        &["SECOND", "SECONDS", "SQL_TSI_SECOND", "SS", "S"],
    ),
    (
        IntervalUnit::Minute,
        &["MINUTE", "MINUTES", "SQL_TSI_MINUTE", "MI", "N"],
    ),
    (
        IntervalUnit::Hour,
        &["HOUR", "HOURS", "SQL_TSI_HOUR", "HH", "H"],
    ),
    (
        IntervalUnit::Day,
        &["DAY", "DAYS", "SQL_TSI_DAY", "DD", "D"],
    ),
    (
        IntervalUnit::Week,
        &["WEEK", "WEEKS", "SQL_TSI_WEEK", "WK", "WW"],
    ),
    (
        IntervalUnit::Month,
        &["MONTH", "MONTHS", "SQL_TSI_MONTH", "MM", "M"],
    ),
    (
        IntervalUnit::Quarter,
        &["QUARTER", "QUARTERS", "SQL_TSI_QUARTER", "QQ", "Q"],
    ),
    (
        IntervalUnit::Year,
        &["YEAR", "YEARS", "SQL_TSI_YEAR", "YYYY", "YY"],
    ),
];

/// Names `ParserDataType` refuses as a type, so a column definition's next clause is not one.
pub(crate) const TYPE_NAME_REFUSED: [&str; 10] = [
    "NOT",
    "NULL",
    "DEFAULT",
    "MATERIALIZED",
    "EPHEMERAL",
    "ALIAS",
    "AUTO",
    "PRIMARY",
    "COMMENT",
    "CODEC",
];

/// Words some type names take after them, in the order `ParserDataType` tries them.
pub(crate) const TYPE_SUFFIXES: [(&str, &[&[&str]]); 6] = [
    (
        "NATIONAL",
        &[
            &["CHARACTER", "LARGE", "OBJECT"],
            &["CHARACTER", "VARYING"],
            &["CHAR", "VARYING"],
            &["CHARACTER"],
            &["CHAR"],
        ],
    ),
    ("BINARY", &[&["LARGE", "OBJECT"], &["VARYING"]]),
    ("CHARACTER", &[&["LARGE", "OBJECT"], &["VARYING"]]),
    ("CHAR", &[&["LARGE", "OBJECT"], &["VARYING"]]),
    ("NCHAR", &[&["LARGE", "OBJECT"], &["VARYING"]]),
    ("DOUBLE", &[&["PRECISION"]]),
];

/// The join kind that may be written alone or before `JOIN`.
pub(crate) const INNER_JOIN: &str = "INNER";

/// The one function name the tuple layer matches with case: `tuple(...)`, not `TUPLE(...)`.
pub(crate) const TUPLE_FUNCTION: &str = "tuple";

/// The boolean keyword that is true; its opposite is `FALSE`.
pub(crate) const TRUE_KEYWORD: &str = "TRUE";

/// The aggregate-state type whose first argument may be a version number.
pub(crate) const VERSIONED_AGGREGATE_TYPE: &str = "AggregateFunction";

/// The parts a column reference holds without dot access: a qualifier and a name.
pub(crate) const COLUMN_NAME_PARTS: usize = 2;
