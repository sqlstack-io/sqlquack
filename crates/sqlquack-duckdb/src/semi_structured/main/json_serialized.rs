//! `json_serialized`: TO_JSON of a VARIANT.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::guards::{bound_with_order, written_value};
use crate::semi_structured::constants::JSON_TEXT;

/// The VARIANT as JSON text, strings quoted and JSON null as `null`.
pub fn json_serialized(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_TEXT);
    bound_with_order(sql, &written_value())
}
