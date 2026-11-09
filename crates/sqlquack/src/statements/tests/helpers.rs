use crate::statements::models::{Outcome, Statement};

/// A statement of `reference` with `outcome`, refused with `code` on `key` when either is given.
pub(crate) fn statement(reference: &str, outcome: Outcome, code: &str, key: &str) -> Statement {
    Statement {
        reference: reference.to_owned(),
        hash: String::new(),
        outcome,
        codes: [code]
            .into_iter()
            .filter(|code| !code.is_empty())
            .map(str::to_owned)
            .collect(),
        refused: [key]
            .into_iter()
            .filter(|key| !key.is_empty())
            .map(str::to_owned)
            .collect(),
        constructs: Vec::new(),
    }
}
