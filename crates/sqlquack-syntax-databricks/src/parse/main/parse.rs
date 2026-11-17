//! `parse`: every statement of a Databricks SQL input, or the error its parser reports.

use sqlquack_syntax::Parsed;

use crate::parse::_helpers::tree::reading::{Reading, parse_as};
use crate::parse::errors::ParseError;

/// Every statement of `sql`; a statement holding a construct the shared tree lacks is a `Command`.
pub fn parse(sql: &str) -> Result<Parsed, ParseError> {
    parse_as(sql, Reading::Trees)
}
