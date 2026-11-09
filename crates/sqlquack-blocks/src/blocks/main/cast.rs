//! `cast`: `CAST(sql AS <duckdb type>)`.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::_helpers::numbers::widened_literal;
use crate::blocks::_helpers::types_map::duckdb_type;
use crate::blocks::constants::CAST;

/// `CAST(sql AS <duckdb type>)`, casting a literal already cast to a narrower DECIMAL only once.
pub fn cast(sql: &str, data_type: SnowflakeType, log: &ProvenanceLog) -> Result<String, String> {
    log.building_block(CAST);
    let sql = match data_type {
        SnowflakeType::Number { precision, scale } => {
            widened_literal(sql, precision, scale).unwrap_or(sql)
        }
        _ => sql,
    };
    Ok(format!("CAST({sql} AS {})", duckdb_type(data_type, log)?))
}
