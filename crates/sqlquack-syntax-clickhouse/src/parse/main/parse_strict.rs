//! `parse_strict`: every statement of a ClickHouse SQL input, or why it does not parse.

use sqlquack_syntax::Parsed;

use crate::parse::errors::ParseError;
use crate::parse::main::parse::parse;

/// `parse`'s statements; every byte is in a token, so a lexical fault is a parse error already.
pub fn parse_strict(sql: &str) -> Result<Parsed, ParseError> {
    parse(sql)
}
