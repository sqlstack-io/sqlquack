//! One guarded parse for any dialect.

use sqlquack_syntax::{CallMode, ComplexityGuardOptions, Dialect, Error, Expression, ParseOptions};

use crate::parsing::constants::MAX_FUNCTION_CALL_DEPTH;
use sqlquack_syntax::tokens::errors::ParseFailure;
use sqlquack_syntax::tokens::models::SourceLocation;

pub(crate) fn parse_guarded(sql: &str, dialect: Dialect) -> Result<Vec<Expression>, ParseFailure> {
    let options = ParseOptions {
        complexity_guard: Some(ComplexityGuardOptions {
            max_function_call_depth: Some(MAX_FUNCTION_CALL_DEPTH),
            ..ComplexityGuardOptions::default()
        }),
    };
    sqlquack_syntax::parse_statements_with(sql, dialect, &options, CallMode::FaithfulLocated)
        .map_err(|error| failure(&error))
}

fn failure(error: &Error) -> ParseFailure {
    let location = match error {
        Error::Tokenize {
            line,
            column,
            start,
            end,
            ..
        }
        | Error::Parse {
            line,
            column,
            start,
            end,
            ..
        }
        | Error::Syntax {
            line,
            column,
            start,
            end,
            ..
        } => Some(SourceLocation {
            start: *start,
            end: *end,
            line: *line,
            column: *column,
        }),
        _ => None,
    };
    let message = match error {
        Error::Tokenize { message, .. }
        | Error::Parse { message, .. }
        | Error::Syntax { message, .. } => message.clone(),
        other => other.to_string(),
    };
    ParseFailure { message, location }
}
