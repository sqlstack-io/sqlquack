//! `quoted_identifier`: a double-quoted identifier.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::QUOTED_IDENTIFIER;

/// A double-quoted identifier.
pub fn quoted_identifier(name: &str, log: &ProvenanceLog) -> String {
    log.building_block(QUOTED_IDENTIFIER);
    format!("\"{}\"", name.replace('"', "\"\""))
}
