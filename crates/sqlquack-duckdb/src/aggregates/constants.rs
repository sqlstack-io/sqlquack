//! Aggregate building block identifiers.

/// MAX_BY and MIN_BY, raising on tied keys with differing values: `duckdb.value_at_extreme`.
pub const VALUE_AT_EXTREME: &str = "duckdb.value_at_extreme";
/// The divergence for MAX_BY and MIN_BY whose extreme key several rows with differing values share.
pub const EXTREME_KEY_TIES: &str = "extreme_key_ties";
/// MAX of JSON-held VARIANTs, raising for kinds whose order no recording shows: `duckdb.json_variant_max`.
pub const JSON_VARIANT_MAX: &str = "duckdb.json_variant_max";
