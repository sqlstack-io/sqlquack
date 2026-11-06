//! `parse_strict`: every statement of a PostgreSQL 18 input, refusing on any parse error or any
//! byte no token, space or comment covers.

use crate::parse::errors::{ParseError, StrictFailure};
use crate::parse::main::parse::parse;
use crate::parse::models::Parsed;
use crate::scan::_helpers::scanner::Scanner;

/// The statements of `sql`, as `parse` gives them, or why strict parsing refuses it: a parse
/// error, whatever its SQLSTATE, or a byte the scanner consumed outside every token.
pub fn parse_strict(sql: &str) -> Result<Parsed, StrictFailure> {
    let parsed: Parsed = parse(sql).map_err(StrictFailure::Parse)?;
    match Scanner::new(sql).uncovered() {
        Some(position) => Err(StrictFailure::Uncovered { position }),
        None => Ok(parsed),
    }
}

impl StrictFailure {
    /// The parse error, when strict parsing refused a parse error.
    pub fn parse_error(&self) -> Option<&ParseError> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Uncovered { .. } => None,
        }
    }
}
