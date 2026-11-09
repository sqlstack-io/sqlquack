//! What a dialect's fingerprint profile says.

use crate::dialects::types::TokenTexts;

/// How one dialect's SQL is split into tokens, and which upper-cased words it reserves.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FingerprintProfile {
    pub(crate) texts: TokenTexts,
    pub(crate) is_reserved: fn(&str) -> bool,
}
