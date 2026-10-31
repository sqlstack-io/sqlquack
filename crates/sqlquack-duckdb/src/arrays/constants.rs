//! Array building block identifiers and messages.

/// ARRAY_SORT of one array: `duckdb.json_array_sort`.
pub const JSON_ARRAY_SORT: &str = "duckdb.json_array_sort";
/// ARRAY_FLATTEN: `duckdb.json_array_flatten`.
pub const JSON_ARRAY_FLATTEN: &str = "duckdb.json_array_flatten";
/// ARRAY_SLICE: `duckdb.json_array_slice`.
pub const JSON_ARRAY_SLICE: &str = "duckdb.json_array_slice";
/// ARRAY_GENERATE_RANGE: `duckdb.json_array_range`.
pub const JSON_ARRAY_RANGE: &str = "duckdb.json_array_range";
/// TO_ARRAY of a VARIANT: `duckdb.json_to_array`.
pub const JSON_TO_ARRAY: &str = "duckdb.json_to_array";
/// The known divergence for ARRAY_SORT orders no recording shows.
pub const ARRAY_SORT_ORDER: &str = "array_sort_order";
/// Snowflake's message for an argument other than an array.
pub const NOT_AN_ARRAY_MESSAGE: &str = "argument is not an array";
/// Snowflake's message for ARRAY_FLATTEN of an element other than an array.
pub const FLATTEN_ELEMENT_MESSAGE: &str = "Not an array: element of the array is not an array";
/// Snowflake's message for ARRAY_GENERATE_RANGE with a step of zero.
pub const ZERO_STEP_MESSAGE: &str = "step size cannot be zero";
/// DuckDB's JSON kinds in the order ARRAY_SORT was recorded putting them, JSON null last.
pub const SORT_KIND_ORDER: &[&[&str]] = &[
    &["BOOLEAN"],
    &["UBIGINT", "BIGINT", "DOUBLE"],
    &["VARCHAR"],
    &["OBJECT"],
    &["ARRAY"],
    &["NULL"],
];
/// The rank of objects in `SORT_KIND_ORDER`.
pub const SORT_OBJECT_RANK: usize = 3;
/// The rank of arrays in `SORT_KIND_ORDER`.
pub const SORT_ARRAY_RANK: usize = 4;
