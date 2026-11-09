//! `snowflake_profile`: how Snowflake SQL is fingerprinted.

use sqlquack_syntax::tokens::models::Dialect;

use crate::dialects::main::shared_texts::shared_texts;
use crate::dialects::models::FingerprintProfile;
use crate::snowflake::profile::_helpers::reserved_keywords::RESERVED_KEYWORDS;

/// Snowflake SQL's fingerprint profile: its tokens read by the shared tokenizer.
pub(crate) fn snowflake_profile() -> FingerprintProfile {
    FingerprintProfile {
        texts: |sql| shared_texts(sql, Dialect::Snowflake),
        is_reserved: |word| RESERVED_KEYWORDS.contains(word),
        folds_in_context: |_, _| false,
    }
}
