//! `full_case_upper`: uPPER with full Unicode case mapping: multi-character mappings first, then DuckDB's upper.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::_helpers::text::expanded;
use crate::blocks::constants::{FULL_CASE_MAPPING, UPPERCASE_EXPANSIONS};

/// UPPER with full Unicode case mapping: multi-character mappings first, then DuckDB's upper.
pub fn full_case_upper(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(FULL_CASE_MAPPING);
    format!("upper({})", expanded(sql, UPPERCASE_EXPANSIONS, log))
}
