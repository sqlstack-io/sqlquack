//! Constants of DuckDB's format profile.

/// DuckDB calls the guide keeps on one line however wide.
pub(crate) const INLINE_CALLS: [&str; 25] = [
    "AVG",
    "CAST",
    "COALESCE",
    "COUNT",
    "CUME_DIST",
    "EPOCH_MS",
    "EXP",
    "FIRST",
    "JSON_EXTRACT",
    "LAST_VALUE",
    "LEN",
    "LENGTH",
    "LOG",
    "LOGICAL_OR",
    "MIN",
    "MODE",
    "NTH_VALUE",
    "NTILE",
    "PERCENTILE_CONT",
    "PERCENTILE_DISC",
    "PERCENT_RANK",
    "RANK",
    "SUM",
    "TRIM",
    "TRY_CAST",
];
/// The bytes DuckDB lexes as operator characters beyond the shared ones.
pub(crate) const OPERATOR_BYTES: &str = "@";
