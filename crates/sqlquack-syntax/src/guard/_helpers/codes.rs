//! Guard: codes.

use crate::tokens::errors::Error;
use crate::tokens::models::Span;

pub(crate) fn parse_guard_error(
    code: &str,
    actual: usize,
    limit: usize,
    span: Option<Span>,
) -> Error {
    let message = format!("{code}: value {actual} exceeds configured limit {limit}");
    if let Some(span) = span {
        Error::parse(message, span)
    } else {
        Error::parse(message, Span::default())
    }
}
