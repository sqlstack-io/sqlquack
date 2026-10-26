//! `zone_checked`: a zoned value that raises when a time zone name does not exist.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::ZONE_CHECKED;
use crate::blocks::main::string_literal::string_literal;

/// The zoned value unchanged, after DuckDB resolves `zone`, which raises for an unknown name.
pub fn zone_checked(sql: &str, zone: &str, log: &ProvenanceLog) -> String {
    log.building_block(ZONE_CHECKED);
    let zone = string_literal(zone, log);
    format!(
        "(CASE WHEN timezone({zone}, CAST('2000-01-01 00:00:00+00' AS TIMESTAMPTZ)) IS NULL THEN NULL ELSE {sql} END)"
    )
}
