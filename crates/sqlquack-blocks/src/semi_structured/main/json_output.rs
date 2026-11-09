//! `json_output`: a VARIANT result column, as Snowflake returns it.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::writing::{Root, sorted_text};
use crate::semi_structured::constants::JSON_OUTPUT;

/// The VARIANT as DuckDB `JSON`, with keys in byte order at every depth.
pub fn json_output(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_OUTPUT);
    format!("CAST({} AS JSON)", sorted_text(sql, Root::Json))
}
