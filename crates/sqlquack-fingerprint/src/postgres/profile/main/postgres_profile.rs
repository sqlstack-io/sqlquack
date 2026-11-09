//! `postgres_profile`: how PostgreSQL SQL is fingerprinted.

use crate::dialects::models::FingerprintProfile;
use crate::postgres::profile::_helpers::reserved_keywords::RESERVED_KEYWORDS;
use crate::postgres::profile::_helpers::texts::postgres_texts;

/// PostgreSQL SQL's fingerprint profile: its tokens read by PostgreSQL's own scanner.
pub(crate) fn postgres_profile() -> FingerprintProfile {
    FingerprintProfile {
        texts: postgres_texts,
        is_reserved: |word| RESERVED_KEYWORDS.contains(word),
    }
}
