//! `json_array_reverse`: ARRAY_REVERSE, another value read as a one-element array.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::arrays::_helpers::elements::{bound, elements, json_array};
use crate::arrays::constants::JSON_ARRAY_REVERSE;
use crate::arrays::main::json_to_array::json_to_array;

/// The elements in reverse order; NULL for SQL or JSON null; another value as itself in an array.
pub fn json_array_reverse(array: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_ARRAY_REVERSE);
    bound(
        &json_to_array(array, log),
        "a",
        &format!(
            "(CASE WHEN a IS NULL THEN NULL ELSE {} END)",
            json_array(&format!("list_reverse({})", elements("a")))
        ),
    )
}
