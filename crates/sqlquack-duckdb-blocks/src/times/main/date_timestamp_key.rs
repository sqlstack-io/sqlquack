//! `date_timestamp_key`: an order key comparing a DATE with a naive timestamp, range safe.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::DUCKDB_FRACTIONAL_PRECISION;
use crate::temporal::main::truncate_timestamp_to_date::truncate_timestamp_to_date;
use crate::times::constants::{DATE_TIMESTAMP_KEY, DAY_MICROSECONDS, DAY_NANOSECONDS};

/// Twice the days since 1970 of a DATE (`clock` None) or a timestamp of that precision, plus one past midnight.
pub fn date_timestamp_key(sql: &str, clock: Option<u8>, log: &ProvenanceLog) -> String {
    log.building_block(DATE_TIMESTAMP_KEY);
    let Some(precision) = clock else {
        return format!("(2 * ({sql} - DATE '1970-01-01'))");
    };
    let past_midnight = match precision > DUCKDB_FRACTIONAL_PRECISION {
        true => format!("epoch_ns({sql}) % {DAY_NANOSECONDS}"),
        false => format!("epoch_us({sql}) % {DAY_MICROSECONDS}"),
    };
    let day = truncate_timestamp_to_date(sql, precision, log);
    format!("(2 * ({day} - DATE '1970-01-01') + CAST({past_midnight} <> 0 AS BIGINT))")
}
