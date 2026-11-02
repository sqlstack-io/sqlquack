//! Array building block identifiers and messages.

/// ARRAY_SORT of one array: `duckdb.json_array_sort`.
pub const JSON_ARRAY_SORT: &str = "duckdb.json_array_sort";
/// ARRAY_FLATTEN: `duckdb.json_array_flatten`.
pub const JSON_ARRAY_FLATTEN: &str = "duckdb.json_array_flatten";
/// ARRAY_INTERSECTION: `duckdb.json_array_intersection`.
pub const JSON_ARRAY_INTERSECTION: &str = "duckdb.json_array_intersection";
/// ARRAYS_OVERLAP: `duckdb.json_arrays_overlap`.
pub const JSON_ARRAYS_OVERLAP: &str = "duckdb.json_arrays_overlap";
/// ARRAY_DISTINCT: `duckdb.json_array_distinct`.
pub const JSON_ARRAY_DISTINCT: &str = "duckdb.json_array_distinct";
/// ARRAY_REMOVE: `duckdb.json_array_remove`.
pub const JSON_ARRAY_REMOVE: &str = "duckdb.json_array_remove";
/// ARRAY_SLICE: `duckdb.json_array_slice`.
pub const JSON_ARRAY_SLICE: &str = "duckdb.json_array_slice";
/// ARRAY_GENERATE_RANGE: `duckdb.json_array_range`.
pub const JSON_ARRAY_RANGE: &str = "duckdb.json_array_range";
/// ARRAY_REVERSE: `duckdb.json_array_reverse`.
pub const JSON_ARRAY_REVERSE: &str = "duckdb.json_array_reverse";
/// ARRAY_MAX and ARRAY_MIN: `duckdb.json_array_extreme`.
pub const JSON_ARRAY_EXTREME: &str = "duckdb.json_array_extreme";
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
/// A VARIANT's sort key: `duckdb.json_order_key`.
pub const JSON_ORDER_KEY: &str = "duckdb.json_order_key";
/// The magnitude from which a double no longer holds every whole number exactly: 2^53.
pub const EXACT_DOUBLE_LIMIT: &str = "9007199254740992";
