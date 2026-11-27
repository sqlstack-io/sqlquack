//! The Databricks conversion rules and the functions they accept.

use sqlquack_diagnostics::failures::models::{ErrorKind, FailureMode};

use crate::databricks::conversion::models::{DatabricksType, Element, Signature};

/// A query block: its source, filter, grouping, ordering and limit.
pub const SELECT_RULE: &str = "rule.databricks.select";
/// The input rows, `VALUES ... AS input(...)`, read as a parenthesised DuckDB `VALUES` list.
pub const FIXTURE_RULE: &str = "rule.databricks.fixture";
/// A column reference.
pub const COLUMN_RULE: &str = "rule.databricks.column";
/// A literal number, string, boolean, date, timestamp or `NULL`.
pub const LITERAL_RULE: &str = "rule.databricks.literal";
/// A cast whose source and target types DuckDB converts the same way.
pub const CAST_RULE: &str = "rule.databricks.cast";
/// `+`, `-` and `*` on numbers, which raise on integer overflow in both engines.
pub const ARITHMETIC_RULE: &str = "rule.databricks.arithmetic";
/// `/`, `%` and `DIV` on numbers, raising on a zero divisor as Databricks does in ANSI mode.
pub const DIVISION_RULE: &str = "rule.databricks.division";
/// `=`, `<>`, `<`, `<=`, `>`, `>=` and `<=>` on values of one type family.
pub const COMPARISON_RULE: &str = "rule.databricks.comparison";
/// `AND`, `OR` and `NOT` on booleans.
pub const BOOLEAN_RULE: &str = "rule.databricks.boolean";
/// `IS [NOT] NULL`.
pub const NULL_TEST_RULE: &str = "rule.databricks.null_test";
/// `[NOT] BETWEEN` on values of one type family.
pub const BETWEEN_RULE: &str = "rule.databricks.between";
/// `[NOT] IN` a list of values of one type family.
pub const IN_LIST_RULE: &str = "rule.databricks.in_list";
/// A searched `CASE` whose branches share one type.
pub const CASE_RULE: &str = "rule.databricks.case";
/// `[NOT] LIKE`, `LIKE ANY` and `LIKE ALL`, a backslash escaping and raising on an escape Databricks rejects.
pub const LIKE_RULE: &str = "rule.databricks.like";
/// The patterns whose every backslash escapes `_`, `%` or a backslash, as an RE2 expression.
pub const LIKE_ESCAPES: &str = r"([^\\]|\\[_%\\])*";
/// The error a pattern Databricks rejects for its escapes raises in DuckDB.
pub const LIKE_ESCAPE_ERROR: &str = "the LIKE pattern has an escape Databricks rejects";
/// `ORDER BY` with the null ordering written out: Databricks puts NULLs first ascending.
pub const ORDER_RULE: &str = "rule.databricks.order";
/// `CONCAT` of strings as `||`, NULL when any argument is NULL.
pub const CONCAT_RULE: &str = "rule.databricks.concat";
/// `TRIM ... FROM` as DuckDB's trim, ltrim or rtrim with the characters written out.
pub const TRIM_RULE: &str = "rule.databricks.trim";
/// `&`, `|`, `~`, `^`, which is XOR in Databricks and DuckDB's `xor`, and shifts, which wrap as in Java.
pub const BITWISE_RULE: &str = "rule.databricks.bitwise";
/// A function DuckDB has under the same or another name with the same results.
pub const SAME_FUNCTION_RULE: &str = "rule.databricks.same_function";
/// A function rewritten for DuckDB, such as NVL2 as a CASE or LEFT with its zero length written out.
pub const REWRITE_RULE: &str = "rule.databricks.rewrite";
/// The digest length SHA2 reads as SHA-256.
pub const SHA2_SHA256_BITS: &str = "256";
/// The digest length SHA2 also reads as SHA-256.
pub const SHA2_DEFAULT_BITS: &str = "0";
/// An aggregate DuckDB computes the same way, its result cast to the type Databricks gives it.
pub const AGGREGATE_RULE: &str = "rule.databricks.aggregate";
/// A date or timestamp function: a field such as DAY or HOUR, or date arithmetic in whole days.
pub const DATE_FUNCTION_RULE: &str = "rule.databricks.date_function";
/// A string function, with Databricks's handling of empty, NULL and out-of-range arguments written out.
pub const STRING_FUNCTION_RULE: &str = "rule.databricks.string_function";
/// A number function such as MOD, CEIL or SIGN, typed and bounded as Databricks types and bounds it.
pub const NUMBER_FUNCTION_RULE: &str = "rule.databricks.number_function";

/// COUNT, the one aggregate with a `*` argument and a DISTINCT form that changes its result.
pub const COUNT_NAME: &str = "COUNT";
/// The fewest arguments a variadic function such as COALESCE takes.
pub const MIN_LIST_ARGUMENTS: usize = 2;
/// The most digits a literal has for one rounding to DOUBLE to match Databricks's exact conversion.
pub const DOUBLE_EXACT_DIGITS: usize = 15;
/// The length of `YYYY-MM-DD`.
pub const DATE_TEXT_LENGTH: usize = 10;
/// The length of `HH:MM:SS`.
pub const CLOCK_TEXT_LENGTH: usize = 8;
/// The most fraction digits of a second both engines keep: microseconds.
pub const MAX_FRACTION_DIGITS: usize = 6;
/// The type the parser keeps as a custom name: a wall-clock timestamp.
pub const TIMESTAMP_NTZ_NAME: &str = "TIMESTAMP_NTZ";
/// The text values both engines read as DOUBLE's special values.
pub const SPECIAL_DOUBLE_TEXTS: &[&str] = &["NaN", "Infinity", "-Infinity"];
/// The most significant bits a FLOAT holds.
pub const FLOAT_MANTISSA_BITS: u32 = 24;
/// The most significant bits a DOUBLE holds.
pub const DOUBLE_MANTISSA_BITS: u32 = 53;
/// The parameter name of the lambdas array rewrites emit.
pub const LAMBDA_ELEMENT: &str = "element";
/// The array elements whose text DuckDB writes as Databricks returns it; strings raise on control characters.
pub const RENDERED_ELEMENTS: &[Element] = &[
    Element::String,
    Element::Boolean,
    Element::TinyInt,
    Element::SmallInt,
    Element::Int,
    Element::BigInt,
];
/// The characters below U+0020, whose JSON escapes no recording shows Databricks writing.
pub const CONTROL_CHARACTERS: &str = r"[\x00-\x1f]";
/// The error a string array holding a control character raises in DuckDB.
pub const CONTROL_ERROR: &str =
    "no recording shows how Databricks writes a control character in an array";
/// How deep a tree may nest for conversion, which recurses once per level.
pub const MAX_TREE_DEPTH: usize = 128;
/// The most digits a Databricks decimal holds.
pub const MAX_DECIMAL_PRECISION: u8 = 38;
/// The decimal type `DECIMAL` without a precision means.
pub const DEFAULT_DECIMAL: DatabricksType = DatabricksType::Decimal {
    precision: 10,
    scale: 0,
};

/// Functions DuckDB computes as Databricks does; case mapping is left out, as Java maps `ß` differently.
pub const SAME_FUNCTIONS: &[(&str, &str, Signature)] = &[
    ("ABS", "abs", Signature::SameNumber),
    ("COALESCE", "coalesce", Signature::SameType),
    ("GREATEST", "greatest", Signature::SameType),
    ("LEAST", "least", Signature::SameType),
    ("LTRIM", "ltrim", Signature::TrimSpaces),
    ("NULLIF", "nullif", Signature::TwoOfOneType),
    ("RTRIM", "rtrim", Signature::TrimSpaces),
    ("TRIM", "trim", Signature::TrimSpaces),
];

/// An overflow, which Databricks raises as `ARITHMETIC_OVERFLOW` or `CAST_OVERFLOW`.
const OVERFLOW: FailureMode = FailureMode {
    kind: ErrorKind::NumericOverflow,
    snowflake: &[],
};
/// A division by zero, which Databricks raises as `DIVIDE_BY_ZERO` or `REMAINDER_BY_ZERO`.
const DIVISION: FailureMode = FailureMode {
    kind: ErrorKind::DivisionByZero,
    snowflake: &[],
};
/// Text no number reads, a malformed cast to a number.
const NUMBER_TEXT: FailureMode = FailureMode {
    kind: ErrorKind::NumberText,
    snowflake: &[],
};
/// Text no date reads, a malformed cast to DATE.
const DATE_TEXT: FailureMode = FailureMode {
    kind: ErrorKind::DateText,
    snowflake: &[],
};
/// Text no timestamp reads, a malformed cast to a timestamp.
const TIMESTAMP_TEXT: FailureMode = FailureMode {
    kind: ErrorKind::TimestampText,
    snowflake: &[],
};
/// How each rule's DuckDB SQL may fail as Databricks does; Databricks errors carry no Snowflake code.
pub const FAILURE_MODES: &[(&str, &[FailureMode])] = &[
    (ARITHMETIC_RULE, &[OVERFLOW]),
    (SAME_FUNCTION_RULE, &[OVERFLOW]),
    (DIVISION_RULE, &[DIVISION, OVERFLOW]),
    (AGGREGATE_RULE, &[OVERFLOW]),
    (DATE_FUNCTION_RULE, &[OVERFLOW]),
    (STRING_FUNCTION_RULE, &[OVERFLOW]),
    (NUMBER_FUNCTION_RULE, &[OVERFLOW, DIVISION]),
    (
        CAST_RULE,
        &[OVERFLOW, NUMBER_TEXT, DATE_TEXT, TIMESTAMP_TEXT],
    ),
];

/// The largest BIGINT, where Java's cast of a larger DOUBLE saturates.
pub const BIGINT_MAX: &str = "9223372036854775807";
/// The smallest BIGINT, where Java's cast of a smaller DOUBLE saturates.
pub const BIGINT_MIN: &str = "-9223372036854775808";
/// The largest argument FACTORIAL takes; it returns NULL above it, as 21! exceeds BIGINT.
pub const FACTORIAL_MAX: u8 = 20;

/// The modulus CHR takes its code by: it returns the character of the code's low byte.
pub const CHR_MODULUS: u16 = 256;
/// The pad LPAD and RPAD use when none is given, a space.
pub const DEFAULT_PAD: &str = "' '";
/// The length SUBSTR takes when none is given, INT's largest.
pub const SUBSTRING_LENGTH: &str = "2147483647";
/// The years whose every second both engines hold as a timestamp; the year on each side holds only part.
pub const TIMESTAMP_YEARS: (i32, i32) = (-290_307, 294_246);
/// Microseconds per second.
pub const MICROS_PER_SECOND: i64 = 1_000_000;
/// Microseconds per millisecond.
pub const MICROS_PER_MILLI: i64 = 1_000;
/// The units TRUNC takes, by DuckDB's date part; weeks start on Monday in both engines.
pub const DATE_TRUNC_UNITS: &[(&str, &[&str])] = &[
    ("year", &["YEAR", "YYYY", "YY"]),
    ("quarter", &["QUARTER"]),
    ("month", &["MONTH", "MM", "MON"]),
    ("week", &["WEEK"]),
];
/// The units DATE_TRUNC takes, by DuckDB's date part.
pub const TIMESTAMP_TRUNC_UNITS: &[(&str, &[&str])] = &[
    ("year", &["YEAR", "YYYY", "YY"]),
    ("quarter", &["QUARTER"]),
    ("month", &["MONTH", "MM", "MON"]),
    ("week", &["WEEK"]),
    ("day", &["DAY", "DD"]),
    ("hour", &["HOUR"]),
    ("minute", &["MINUTE"]),
    ("second", &["SECOND"]),
    ("millisecond", &["MILLISECOND"]),
    ("microsecond", &["MICROSECOND"]),
];
/// The date day counts start from.
pub const EPOCH_DATE: &str = "1970-01-01";
/// 2^31: an INT day count wraps past it, as Databricks's DATE_ADD does.
pub const INT_RANGE: u64 = 2_147_483_648;
/// 2^32, the span of INT values.
pub const INT_SPAN: u64 = 4_294_967_296;

/// The aggregates converted, with the DuckDB function computing each.
pub const AGGREGATES: &[(&str, &str)] = &[
    ("COUNT", "count"),
    ("MIN", "min"),
    ("MAX", "max"),
    ("SUM", "sum"),
    ("BOOL_AND", "bool_and"),
    ("EVERY", "bool_and"),
    ("BOOL_OR", "bool_or"),
    ("SOME", "bool_or"),
    ("ANY", "bool_or"),
    ("COUNT_IF", "count_if"),
    ("BIT_AND", "bit_and"),
    ("BIT_OR", "bit_or"),
    ("BIT_XOR", "bit_xor"),
];

/// The functions named after a type, which cast their one argument to it.
pub const CAST_FUNCTIONS: &[(&str, DatabricksType)] = &[
    ("BOOLEAN", DatabricksType::Boolean),
    ("TINYINT", DatabricksType::TinyInt),
    ("SMALLINT", DatabricksType::SmallInt),
    ("INT", DatabricksType::Int),
    ("BIGINT", DatabricksType::BigInt),
    ("FLOAT", DatabricksType::Float),
    ("DOUBLE", DatabricksType::Double),
    ("DECIMAL", DEFAULT_DECIMAL),
    ("STRING", DatabricksType::String),
    ("DATE", DatabricksType::Date),
    ("TIMESTAMP", DatabricksType::Timestamp),
];
