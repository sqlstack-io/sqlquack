//! `full_case_lower`: lOWER with full Unicode case mapping: multi-character mappings first, then DuckDB's lower.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::_helpers::text::expanded;
use crate::blocks::constants::{FULL_CASE_MAPPING, LOWERCASE_EXPANSIONS};

/// LOWER with full Unicode case mapping: multi-character mappings first, then DuckDB's lower.
pub fn full_case_lower(sql: &str, log: &ProvenanceLog) -> String {
    log.building_block(FULL_CASE_MAPPING);
    format!("lower({})", expanded(sql, LOWERCASE_EXPANSIONS, log))
}
