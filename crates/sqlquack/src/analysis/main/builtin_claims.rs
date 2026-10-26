//! `builtin_claims`: everything the built-in converter relies on that cases must verify.

use sqlquack_evidence::verify::models::Claim;
use sqlquack_rules::conversion::constants::{RULES, SPELLING_PAIRS};

/// Every recorded spelling pair and every conversion rule.
pub fn builtin_claims() -> Vec<Claim> {
    SPELLING_PAIRS
        .iter()
        .map(|pair| Claim::Equivalence(pair.id.to_owned()))
        .chain(RULES.iter().map(|rule| Claim::Rule(rule.id.to_owned())))
        .collect()
}
