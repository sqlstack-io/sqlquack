//! `utc_to_zone`: a naive UTC wall time as the wall time in another zone.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::UTC_TO_ZONE;
use crate::blocks::main::string_literal::string_literal;
use crate::temporal::_helpers::timestamps::{microsecond_base, with_source_nanos};

/// The naive timestamp read as UTC and shown as the wall time in `zone`, at its precision.
pub fn utc_to_zone(sql: &str, zone: &str, source: SnowflakeType, log: &ProvenanceLog) -> String {
    log.building_block(UTC_TO_ZONE);
    let base = microsecond_base(sql, source);
    let zone = string_literal(zone, log);
    with_source_nanos(
        &format!("timezone({zone}, timezone('UTC', {base}))"),
        sql,
        source,
    )
}
