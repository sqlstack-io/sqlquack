//! `parse_strict_reviewed`: `parse_strict` for readers that lay SQL out.

use sqlquack_syntax::Parsed;

use crate::parse::_helpers::tree::reading::{Reading, strict};
use crate::parse::errors::StrictFailure;

/// `parse_strict`'s statements, with every form of `TREED_FORMS` kept as a gap, so it is refused.
pub fn parse_strict_reviewed(sql: &str) -> Result<Parsed, StrictFailure> {
    strict(sql, Reading::Reviewed)
}
