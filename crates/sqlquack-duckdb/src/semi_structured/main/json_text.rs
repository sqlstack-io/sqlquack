//! `json_text`: a VARIANT cast to VARCHAR, as Snowflake writes it.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::guards::{bound_with_order, written_value};
use crate::semi_structured::constants::JSON_TEXT;

/// Strings unquoted, JSON null as NULL, other values as JSON text with keys in byte order.
pub fn json_text(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_TEXT);
    bound_with_order(
        sql,
        &format!(
            "(CASE json_type(r['j']) WHEN 'NULL' THEN NULL \
             WHEN 'VARCHAR' THEN json_extract_string(r['j'], '$') ELSE {} END)",
            written_value()
        ),
    )
}
