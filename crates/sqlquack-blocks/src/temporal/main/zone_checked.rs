//! `zone_checked`: a zoned value that raises when a time zone name does not exist.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::ZONE_CHECKED;

/// The zoned value unchanged, after DuckDB resolves `zone`, which raises for an unknown name.
pub fn zone_checked(sql: &str, zone: &str, log: &ProvenanceLog) -> String {
    log.building_block(ZONE_CHECKED);
    format!(
        "(CASE WHEN timezone({zone}, CAST('2000-01-01 00:00:00+00' AS TIMESTAMPTZ)) IS NULL THEN NULL ELSE {sql} END)"
    )
}
