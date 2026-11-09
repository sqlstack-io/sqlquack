//! `parse_strict`: every statement of a PostgreSQL 18 input, refusing any error or uncovered byte.

use crate::parse::errors::StrictFailure;
use crate::parse::main::parse::parse;
use crate::scan::main::uncovered::uncovered;
use sqlquack_syntax::Parsed;

/// `parse`'s statements, refused on any parse error, whatever its SQLSTATE, or uncovered byte.
pub fn parse_strict(sql: &str) -> Result<Parsed, StrictFailure> {
    let parsed: Parsed = parse(sql).map_err(StrictFailure::Parse)?;
    match uncovered(sql) {
        Some(position) => Err(StrictFailure::Uncovered { position }),
        None => Ok(parsed),
    }
}
