//! `hash_value`: a signed 64-bit hash of several values, consistent within one DuckDB run.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::hashing::constants::{HASH_OFFSET, HASH_VALUE};

/// DuckDB's hash of the values in order, shifted into the signed range as a DECIMAL(19,0).
pub fn hash_value(values: &[String], log: &ProvenanceLog) -> String {
    log.building_block(HASH_VALUE);
    format!(
        "CAST(CAST(hash({}) AS HUGEINT) - {HASH_OFFSET} AS DECIMAL(19,0))",
        values.join(", ")
    )
}
