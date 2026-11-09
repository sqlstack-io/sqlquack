//! The values DuckDB's profile and lineage rules read.

/// The width of DuckDB's BIGINT.
pub(crate) const DUCKDB_BIGINT_BITS: u16 = 64;
/// The widest DuckDB DECIMAL.
pub(crate) const DUCKDB_DECIMAL_LIMIT: u32 = 38;
/// DuckDB's DECIMAL without a precision.
pub(crate) const DUCKDB_DEFAULT_DECIMAL: (u32, u32) = (18, 3);
/// DuckDB's date part counting seconds since the epoch, a DOUBLE.
pub(crate) const DUCKDB_EPOCH_PART: &str = "epoch";
/// Every spelling of each DuckDB integer type.
pub(crate) const DUCKDB_INTEGER_ALIASES: [(&[&str], (u16, bool)); 10] = [
    (&["TINYINT", "INT1"], (8, false)),
    (&["SMALLINT", "INT2", "SHORT"], (16, false)),
    (&["INTEGER", "INT", "INT4", "SIGNED"], (32, false)),
    (&["BIGINT", "INT8", "LONG"], (64, false)),
    (&["HUGEINT", "INT128"], (128, false)),
    (&["UTINYINT", "UINT8"], (8, true)),
    (&["USMALLINT", "UINT16"], (16, true)),
    (&["UINTEGER", "UINT32"], (32, true)),
    (&["UBIGINT", "UINT64"], (64, true)),
    (&["UHUGEINT", "UINT128"], (128, true)),
];
/// The width of DuckDB's INTEGER.
pub(crate) const DUCKDB_INTEGER_BITS: u16 = 32;
/// DuckDB's spelling of each integer type: (bits, unsigned, spelling).
pub(crate) const DUCKDB_INTEGER_SPELLINGS: [(u16, bool, &str); 10] = [
    (8, false, "TINYINT"),
    (16, false, "SMALLINT"),
    (32, false, "INTEGER"),
    (64, false, "BIGINT"),
    (128, false, "HUGEINT"),
    (8, true, "UTINYINT"),
    (16, true, "USMALLINT"),
    (32, true, "UINTEGER"),
    (64, true, "UBIGINT"),
    (128, true, "UHUGEINT"),
];
/// Semi-structured DuckDB types CONCAT does not read as text.
pub(crate) const DUCKDB_JSON_TYPES: [&str; 2] = ["JSON", "JSONB"];
/// DuckDB's Julian day date part, a DOUBLE.
pub(crate) const DUCKDB_JULIAN_PART: &str = "julian";
/// The widest DECIMAL DuckDB keeps in 64 bits.
pub(crate) const DUCKDB_SMALL_DECIMAL: u32 = 18;
/// The widest DuckDB integers, HUGEINT and UHUGEINT.
pub(crate) const DUCKDB_WIDEST_INTEGER: u16 = 128;
/// The arguments of a one-argument function.
pub(crate) const ONE_ARGUMENT: usize = 1;
/// The arguments of a two-argument function.
pub(crate) const TWO_ARGUMENTS: usize = 2;
/// DuckDB's built-in table functions of one column, named after the function.
pub(crate) const DUCKDB_TABLE_FUNCTION_COLUMNS: [(&str, &[&str]); 3] = [
    ("GENERATE_SERIES", &["generate_series"]),
    ("RANGE", &["range"]),
    ("UNNEST", &[DUCKDB_UNNEST_COLUMN]),
];
/// The one column DuckDB's UNNEST in FROM returns.
pub(crate) const DUCKDB_UNNEST_COLUMN: &str = "unnest";
/// DuckDB's name for the n-th column of a VALUES list, numbered from 1.
pub(crate) const DUCKDB_VALUES_COLUMN: &str = "col";
