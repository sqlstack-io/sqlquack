//! `refused_construct`: the construct key a refusal names.

use sqlquack_diagnostics::refusals::models::Refusal;

use crate::analysis::constants::STATEMENT_REFUSAL;

/// The construct key a refusal names, or `statement`.
pub fn refused_construct(refusal: &Refusal) -> String {
    refusal
        .construct
        .as_ref()
        .map_or_else(|| STATEMENT_REFUSAL.to_owned(), |construct| construct.key())
}
