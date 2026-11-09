//! What a dialect's fingerprint profile says.

use crate::dialects::types::TokenTexts;

/// How one dialect's SQL is split into tokens, which upper-cased words it reserves, and where its other keywords fold.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FingerprintProfile {
    pub(crate) texts: TokenTexts,
    pub(crate) is_reserved: fn(&str) -> bool,
    /// Whether the upper-cased word at an index, not reserved, still folds there because it acts as a keyword.
    pub(crate) folds_in_context: fn(&[String], usize) -> bool,
}
