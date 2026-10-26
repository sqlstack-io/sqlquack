//! `json_hash_key`: the text a VARIANT is hashed by, the same for every equal Snowflake value.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::semi_structured::_helpers::guards::{bound_with_order, written_value};
use crate::semi_structured::constants::JSON_HASH_KEY;

/// The JSON text with keys in byte order, raising where DuckDB keeps them in another order.
pub fn json_hash_key(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(JSON_HASH_KEY);
    bound_with_order(sql, &written_value())
}
