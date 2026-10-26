//! `column_type`: the DuckDB column type a Snowflake type is held in.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::blocks::_helpers::types_map::duckdb_type;

/// The DuckDB type of a column holding `data_type`, or why there is none.
pub fn column_type(data_type: SnowflakeType, log: &ProvenanceLog) -> Result<String, String> {
    duckdb_type(data_type, log)
}
