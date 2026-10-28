//! `epoch_to_timestamp`: an integer count since the epoch as a naive timestamp.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::{
    DUCKDB_FRACTIONAL_PRECISION, EPOCH_SECONDS_MAX, EPOCH_SECONDS_MIN, EPOCH_TO_TIMESTAMP,
    NANOSECOND_DIGITS, TIMESTAMP_RANGE,
};
use crate::raises::main::raised::raised;

/// `sql` units of 10^-`scale` seconds since the epoch; counts beyond years 1-9999 raise.
pub fn epoch_to_timestamp(sql: &str, scale: u8, log: &ProvenanceLog) -> String {
    log.building_block(EPOCH_TO_TIMESTAMP);
    let per_second = 10_i128.pow(u32::from(scale));
    let (mut low, mut high) = (
        EPOCH_SECONDS_MIN * per_second,
        (EPOCH_SECONDS_MAX + 1) * per_second - 1,
    );
    if scale > DUCKDB_FRACTIONAL_PRECISION {
        let per_unit = 10_i128.pow(u32::from(NANOSECOND_DIGITS - scale));
        low = low.max(i128::from(i64::MIN) / per_unit + 1);
        high = high.min(i128::from(i64::MAX) / per_unit);
    }
    let converted = match scale <= DUCKDB_FRACTIONAL_PRECISION {
        true => format!(
            "make_timestamp(CAST(epoch_units AS BIGINT) * {})",
            10_i64.pow(u32::from(DUCKDB_FRACTIONAL_PRECISION - scale))
        ),
        false => format!(
            "make_timestamp_ns(CAST(epoch_units AS BIGINT) * {})",
            10_i64.pow(u32::from(NANOSECOND_DIGITS - scale))
        ),
    };
    format!(
        "list_transform([{sql}], lambda epoch_units: CASE WHEN epoch_units < {low} \
         OR epoch_units > {high} THEN {} ELSE {converted} END)[1]",
        raised(&format!(
            "{TIMESTAMP_RANGE}: an epoch count outside the years 1 to 9999 or DuckDB's range"
        ))
    )
}
