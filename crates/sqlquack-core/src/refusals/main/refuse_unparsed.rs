//! `refuse_unparsed`: the refusal for SQL that does not parse.

use crate::parsing::errors::ParseFailure;
use crate::refusals::models::{Refusal, RefusalCategory};

/// Refuses a whole input that polyglot could not parse.
pub fn refuse_unparsed(failure: &ParseFailure) -> Refusal {
    Refusal {
        construct: None,
        location: failure.location,
        category: RefusalCategory::ParseFidelity,
        reason: format!("the SQL does not parse: {}", failure.message),
    }
}
