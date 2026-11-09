//! `builtin_claims`: everything the built-in converter relies on that cases must verify.

use sqlquack_convert::snowflake::catalogue::constants::RULES;
use sqlquack_convert::snowflake::conversion::constants::SPELLING_PAIRS;
use sqlquack_evidence::verify::models::Claim;

/// Every recorded spelling pair and every conversion rule.
pub fn builtin_claims() -> Vec<Claim> {
    SPELLING_PAIRS
        .iter()
        .map(|pair| Claim::Equivalence(pair.id.to_owned()))
        .chain(RULES.iter().map(|rule| Claim::Rule(rule.id.to_owned())))
        .collect()
}
