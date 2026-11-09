//! `json_array_size`: ARRAY_SIZE of a VARIANT.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_ARRAY_SIZE;
use crate::variants::_helpers::objects::bound;

/// The array's length as a NUMBER(9,0); NULL, JSON null and values other than arrays give NULL.
pub fn json_array_size(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_SIZE);
    bound(
        sql,
        "a",
        "(CASE WHEN json_type(a) = 'ARRAY' THEN CAST(json_array_length(a) AS DECIMAL(9,0)) END)",
    )
}
