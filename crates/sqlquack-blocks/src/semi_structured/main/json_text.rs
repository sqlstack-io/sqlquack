//! `json_text`: a VARIANT cast to VARCHAR, as Snowflake writes it.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::writing::{Root, sorted_text};
use crate::semi_structured::constants::JSON_TEXT;

/// Strings unquoted, JSON null as NULL, other values as JSON text with keys in byte order.
pub fn json_text(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_TEXT);
    sorted_text(sql, Root::Text)
}
