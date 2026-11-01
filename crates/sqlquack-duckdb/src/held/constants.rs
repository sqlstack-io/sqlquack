//! Building block identifiers of VARIANT-held values, and the patterns their writer uses.

/// JSON of typed leaves held as a DuckDB VARIANT: `duckdb.variant_built`.
pub const VARIANT_BUILT: &str = "duckdb.variant_built";
/// A FLOAT as a leaf, raising for NaN, the infinities and negative zero: `duckdb.variant_double_leaf`.
pub const VARIANT_DOUBLE_LEAF: &str = "duckdb.variant_double_leaf";
/// A whole NUMBER as a leaf, raising beyond DuckDB's integers: `duckdb.variant_integer_leaf`.
pub const VARIANT_INTEGER_LEAF: &str = "duckdb.variant_integer_leaf";
/// A NUMBER with a scale as a held VARIANT leaf: `duckdb.variant_decimal_leaf`.
pub const VARIANT_DECIMAL_LEAF: &str = "duckdb.variant_decimal_leaf";
/// A JSON-held value as a leaf, raising for retyped numbers: `duckdb.variant_json_leaf`.
pub const VARIANT_JSON_LEAF: &str = "duckdb.variant_json_leaf";
/// A VARIANT-held value as Snowflake writes it as JSON: `duckdb.variant_written`.
pub const VARIANT_WRITTEN: &str = "duckdb.variant_written";
/// A VARIANT-held value cast to text: `duckdb.variant_text`.
pub const VARIANT_TEXT: &str = "duckdb.variant_text";
/// Snowflake's TYPEOF of a VARIANT-held value: `duckdb.variant_type_name`.
pub const VARIANT_TYPE_NAME: &str = "duckdb.variant_type_name";
/// A VARIANT-held value cast to FLOAT: `duckdb.variant_double`.
pub const VARIANT_DOUBLE: &str = "duckdb.variant_double";
/// A VARIANT-held value read as JSON, for paths and sizes: `duckdb.variant_json`.
pub const VARIANT_JSON: &str = "duckdb.variant_json";

/// A JSON string or number token, as RE2 matches it.
pub const HELD_TOKEN_PATTERN: &str = r#""(?:[^"\\]|\\.)*"|-?[0-9][0-9.eE+-]*"#;
/// A number token DuckDB writes for a double: with a point or an exponent.
pub const HELD_DOUBLE_TOKEN_PATTERN: &str = "[.eE]";
/// The `printf` format Snowflake writes a double inside a VARIANT with.
pub const HELD_DOUBLE_FORMAT: &str = "%.15e";
/// A number DuckDB's VARIANT would not hold as written: a point, an exponent or beyond 64 bits.
pub const HELD_RETYPED_NUMBER_PATTERN: &str = "[.eE]|-[0-9]{19}|[0-9]{20}";
/// The smallest whole NUMBER DuckDB's VARIANT holds as an integer.
pub const HELD_INTEGER_MIN: &str = "-9223372036854775808";
/// The largest whole NUMBER DuckDB's VARIANT holds as an integer.
pub const HELD_INTEGER_MAX: &str = "18446744073709551615";
