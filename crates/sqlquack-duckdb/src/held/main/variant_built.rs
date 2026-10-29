//! `variant_built`: JSON of typed leaves, held as a DuckDB VARIANT.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::held::constants::VARIANT_BUILT;

/// The JSON `sql`, whose leaves are typed as `variant_*_leaf` writes them, as a DuckDB VARIANT.
pub fn variant_built(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(VARIANT_BUILT);
    format!("CAST({sql} AS VARIANT)")
}
