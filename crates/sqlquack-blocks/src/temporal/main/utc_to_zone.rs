//! `utc_to_zone`: a naive UTC wall time as the wall time in another zone.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::UTC_TO_ZONE;
use crate::temporal::_helpers::operands::once;
use crate::temporal::_helpers::timestamps::{microsecond_base, with_source_nanos};

/// The naive timestamp read as UTC and shown as the wall time in `zone`, at its precision.
pub fn utc_to_zone(value: &str, zone: &str, source: SnowflakeType, log: &ProvenanceLog) -> String {
    log.building_block(UTC_TO_ZONE);
    once(value, "utc_wall", |sql| converted(sql, zone, source))
}

/// The conversion of the plain operand `sql`.
fn converted(sql: &str, zone: &str, source: SnowflakeType) -> String {
    let base = microsecond_base(sql, source);
    with_source_nanos(
        &format!("timezone({zone}, timezone('UTC', {base}))"),
        sql,
        source,
    )
}
