//! `json_null_value`: IS_NULL_VALUE of a VARIANT.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_TYPE_NAME;

/// Whether the VARIANT is JSON null; NULL for SQL NULL.
pub fn json_null_value(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_TYPE_NAME);
    format!("(json_type({sql}) = 'NULL')")
}
