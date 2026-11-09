//! `boolean_to_number`: bOOLEAN to NUMBER: true is 1.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::blocks::constants::BOOLEAN_NUMBER;
use crate::blocks::main::cast::cast;

/// BOOLEAN to NUMBER: true is 1.
pub fn boolean_to_number(
    sql: &str,
    result: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(BOOLEAN_NUMBER);
    cast(&format!("CAST({sql} AS INTEGER)"), result, log)
}
