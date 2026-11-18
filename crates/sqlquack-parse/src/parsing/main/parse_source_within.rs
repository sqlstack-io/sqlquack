//! `parse_source_within`: source SQL to statements, with the caller's function-call limit.

use sqlquack_syntax::tokens::errors::ParseFailure;
use sqlquack_syntax::{Dialect, Expression};

use crate::parsing::_helpers::guarded::parse_guarded;
use crate::parsing::constants::MAX_FUNCTION_CALL_DEPTH;
use crate::parsing::main::call_depth_hint::call_depth_hint;
use crate::parsing::main::checked_call_depth::checked_call_depth;

/// Parses source SQL as `parse_source` does, refusing calls nested deeper than `max_call_depth`.
pub fn parse_source_within(
    sql: &str,
    dialect: Dialect,
    max_call_depth: Option<usize>,
) -> Result<Vec<Expression>, ParseFailure> {
    let limit = checked_call_depth(max_call_depth.unwrap_or(MAX_FUNCTION_CALL_DEPTH)).map_err(
        |message| ParseFailure {
            message,
            location: None,
        },
    )?;
    parse_guarded(sql, dialect, limit).map_err(|failure| ParseFailure {
        message: call_depth_hint(failure.message, limit, None),
        ..failure
    })
}
