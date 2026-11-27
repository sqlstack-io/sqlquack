//! The Databricks conversion rules and the functions they accept.

use crate::databricks::conversion::models::{DatabricksType, Signature};

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
/// `/` and `%` by a constant other than zero, so Databricks never raises where DuckDB returns NULL.
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
/// `[NOT] LIKE`, `LIKE ANY` and `LIKE ALL` with constant patterns holding no backslash.
pub const LIKE_RULE: &str = "rule.databricks.like";
/// `ORDER BY` with the null ordering written out: Databricks puts NULLs first ascending.
pub const ORDER_RULE: &str = "rule.databricks.order";
/// `CONCAT` of strings as `||`, NULL when any argument is NULL.
pub const CONCAT_RULE: &str = "rule.databricks.concat";
/// `TRIM ... FROM` as DuckDB's trim, ltrim or rtrim with the characters written out.
pub const TRIM_RULE: &str = "rule.databricks.trim";
/// `&`, `|`, `~`, and `^`, which is XOR in Databricks and DuckDB's `xor`.
pub const BITWISE_RULE: &str = "rule.databricks.bitwise";
/// A function DuckDB has under the same or another name with the same results.
pub const SAME_FUNCTION_RULE: &str = "rule.databricks.same_function";
/// A function rewritten for DuckDB, such as NVL2 as a CASE or LEFT with its zero length written out.
pub const REWRITE_RULE: &str = "rule.databricks.rewrite";
/// The digest length SHA2 reads as SHA-256.
pub const SHA2_SHA256_BITS: &str = "256";
/// The digest length SHA2 also reads as SHA-256.
pub const SHA2_DEFAULT_BITS: &str = "0";
/// An aggregate DuckDB computes the same way.
pub const AGGREGATE_RULE: &str = "rule.databricks.aggregate";

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
    ("MD5", "md5", Signature::StringToString),
    ("NULLIF", "nullif", Signature::TwoOfOneType),
    ("RTRIM", "rtrim", Signature::TrimSpaces),
    ("SHA1", "sha1", Signature::StringToString),
    ("TRIM", "trim", Signature::TrimSpaces),
];
