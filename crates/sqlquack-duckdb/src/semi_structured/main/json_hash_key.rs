//! `json_hash_key`: the text a VARIANT is hashed by, the same for every equal Snowflake value.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::writing::{Root, sorted_text};
use crate::semi_structured::constants::JSON_HASH_KEY;

/// The JSON text with keys in byte order at every depth.
pub fn json_hash_key(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_HASH_KEY);
    sorted_text(sql, Root::Json)
}
