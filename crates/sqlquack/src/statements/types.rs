//! Type aliases of the statements file.

use crate::statements::models::Outcome;

/// A case's outcome, its refusal codes, and its refused constructs.
pub type Judged = (Outcome, Vec<String>, Vec<String>);
