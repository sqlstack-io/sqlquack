//! `cast`: `CAST(sql AS <duckdb type>)`.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::blocks::_helpers::types_map::duckdb_type;
use crate::blocks::constants::CAST;

/// `CAST(sql AS <duckdb type>)`.
pub fn cast(sql: &str, data_type: SnowflakeType, log: &ProvenanceLog) -> Result<String, String> {
    log.building_block(CAST);
    Ok(format!("CAST({sql} AS {})", duckdb_type(data_type, log)?))
}
