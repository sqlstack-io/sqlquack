//! Aggregate building block identifiers.

/// MAX_BY and MIN_BY, raising on tied keys with differing values: `duckdb.value_at_extreme`.
pub const VALUE_AT_EXTREME: &str = "duckdb.value_at_extreme";
/// The divergence for MAX_BY and MIN_BY whose extreme key several rows with differing values share.
pub const EXTREME_KEY_TIES: &str = "extreme_key_ties";
/// MEDIAN: `duckdb.list_median`.
pub const LIST_MEDIAN: &str = "duckdb.list_median";
/// The divergence for interpolated percentiles and MEDIANs no recording reproduces.
pub const PERCENTILE_INTERPOLATION: &str = "percentile_interpolation";
/// MIN or MAX of JSON-held VARIANTs: `duckdb.json_variant_extreme`.
pub const JSON_VARIANT_EXTREME: &str = "duckdb.json_variant_extreme";
