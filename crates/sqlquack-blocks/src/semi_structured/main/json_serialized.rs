//! `json_serialized`: TO_JSON of a VARIANT.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::writing::{Root, sorted_text};
use crate::semi_structured::constants::JSON_TEXT;

/// The VARIANT as JSON text with keys in byte order, strings quoted and JSON null as `null`.
pub fn json_serialized(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_TEXT);
    sorted_text(sql, Root::Json)
}
