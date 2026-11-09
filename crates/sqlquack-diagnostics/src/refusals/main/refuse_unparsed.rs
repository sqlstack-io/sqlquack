//! `refuse_unparsed`: the refusal for SQL that does not parse.

use crate::diagnostics::constants::UNPARSED_PREFIX;
use crate::refusals::models::{Refusal, RefusalCategory};
use sqlquack_syntax::tokens::errors::ParseFailure;
use sqlquack_syntax::tokens::models::SourceLines;

/// Refuses a whole input that could not be parsed, at the failure or else the whole source.
pub fn refuse_unparsed(failure: &ParseFailure, lines: &SourceLines) -> Refusal {
    let location = match failure.location {
        Some(location) => lines.location(location.start, location.end),
        None => lines.whole(),
    };
    Refusal {
        construct: None,
        location: Some(location),
        category: RefusalCategory::ParseFidelity,
        reason: format!("{UNPARSED_PREFIX}{}", failure.message),
    }
}
