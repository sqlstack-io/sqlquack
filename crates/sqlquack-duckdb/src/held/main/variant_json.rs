//! `variant_json`: a VARIANT-held value read as JSON.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::held::constants::VARIANT_JSON;

/// The VARIANT-held `sql` as DuckDB JSON: keys in byte order, doubles with a point or exponent.
pub fn variant_json(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_JSON);
    format!("CAST({sql} AS JSON)")
}
