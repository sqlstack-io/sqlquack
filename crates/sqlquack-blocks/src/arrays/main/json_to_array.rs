//! `json_to_array`: TO_ARRAY of a VARIANT, an array kept and another value wrapped.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::bound;
use crate::arrays::constants::JSON_TO_ARRAY;

/// The array itself, NULL for SQL or JSON null, and any other value as a one-element array.
pub fn json_to_array(value: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_TO_ARRAY);
    bound(
        &format!("CAST({value} AS JSON)"),
        "q",
        "(CASE WHEN q IS NULL OR json_type(q) = 'NULL' THEN NULL WHEN json_type(q) = 'ARRAY' THEN q \
         ELSE CAST(to_json([q]) AS JSON) END)",
    )
}
