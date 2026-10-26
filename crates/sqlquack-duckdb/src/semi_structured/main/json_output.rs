//! `json_output`: a VARIANT result column, as Snowflake returns it.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::guards::{bound_with_order, written_value};
use crate::semi_structured::constants::JSON_OUTPUT;

/// The VARIANT as DuckDB `JSON`, raising for objects whose keys Snowflake would reorder.
pub fn json_output(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_OUTPUT);
    format!("CAST({} AS JSON)", bound_with_order(sql, &written_value()))
}
