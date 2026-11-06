//! `binary_length_check`: raises an error when bytes are longer than `max_length`, as a BINARY(n) cast does.

use sqlquack_core::failures::constants::BINARY_TOO_LONG;
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::raises::main::raised::raised;
use crate::text::constants::{BINARY_LENGTH_CHECK, BINARY_TOO_LONG_MESSAGE};

/// Raises an error when bytes are longer than `max_length`, as a BINARY(n) cast does.
pub fn binary_length_check(sql: &str, max_length: u32, log: &ProvenanceLog) -> String {
    log.building_block(BINARY_LENGTH_CHECK);
    format!(
        "list_transform([{sql}], lambda binary_value: CASE \
         WHEN octet_length(binary_value) > {max_length} THEN CAST({} AS BLOB) \
         ELSE binary_value END)[1]",
        raised(&tagged_message(BINARY_TOO_LONG, BINARY_TOO_LONG_MESSAGE))
    )
}
