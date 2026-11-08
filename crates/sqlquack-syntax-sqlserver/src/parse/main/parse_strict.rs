//! `parse_strict`: every statement of a Transact-SQL input, refusing any error.

use crate::parse::errors::StrictFailure;
use crate::parse::main::parse::parse;
use crate::parse::models::Parsed;

/// `parse`'s statements, refused on any parse error, syntax or not.
pub fn parse_strict(sql: &str) -> Result<Parsed, StrictFailure> {
    parse(sql).map_err(StrictFailure::Parse)
}
