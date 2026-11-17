//! `parse_strict`: every statement of a Databricks SQL input, refusing any error or uncovered byte.

use sqlquack_syntax::Parsed;

use crate::parse::_helpers::tree::reading::{Reading, strict};
use crate::parse::errors::StrictFailure;

/// `parse`'s statements, refused on any parse error, syntax or not, or uncovered byte.
pub fn parse_strict(sql: &str) -> Result<Parsed, StrictFailure> {
    strict(sql, Reading::Trees)
}
