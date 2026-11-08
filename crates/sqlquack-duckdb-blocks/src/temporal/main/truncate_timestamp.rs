//! `truncate_timestamp`: a naive timestamp cut to fewer fractional digits, as Snowflake truncates.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::TRUNCATE_TIMESTAMP;
use crate::temporal::_helpers::timestamps::{epoch_units, floor_units, from_units, stored_digits};

/// The naive timestamp at `from` digits truncated toward the past to `to` digits; unchanged when wider.
pub fn truncate_timestamp(sql: &str, (from, to): (u8, u8), log: &ProvenanceLog) -> String {
    log.building_block(TRUNCATE_TIMESTAMP);
    let source = stored_digits(from);
    let target = stored_digits(to);
    let units = floor_units(&epoch_units(sql, source), source, to);
    match to >= from {
        true => sql.to_owned(),
        false => from_units(&units, to, target),
    }
}
