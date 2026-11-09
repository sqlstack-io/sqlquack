//! `json_is_kind`: IS_OBJECT, IS_ARRAY and IS_BOOLEAN of a VARIANT.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::semi_structured::constants::JSON_IS_KIND;
use crate::variants::_helpers::objects::bound;

/// Whether the VARIANT holds a value of DuckDB JSON type `kind`; SQL NULL stays NULL.
pub fn json_is_kind(sql: &str, kind: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_IS_KIND);
    bound(
        sql,
        "j",
        &format!("(CASE WHEN j IS NULL THEN NULL ELSE json_type(j) = '{kind}' END)"),
    )
}
