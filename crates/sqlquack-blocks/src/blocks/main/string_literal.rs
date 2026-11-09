//! `string_literal`: a single-quoted string literal.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::STRING_LITERAL;

/// A single-quoted string literal.
pub fn string_literal(text: &str, log: &ProvenanceLog) -> String {
    log.building_block(STRING_LITERAL);
    format!("'{}'", text.replace('\'', "''"))
}
