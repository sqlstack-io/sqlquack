//! `sqlserver_profile`: how SQL Server SQL is fingerprinted.

use sqlquack_parse::sqlserver::keywords::main::is_reserved::is_reserved;

use crate::dialects::models::FingerprintProfile;
use crate::sqlserver::profile::_helpers::texts::sqlserver_texts;

/// SQL Server SQL's fingerprint profile: its tokens read by SQL Server's own scanner, with its parser's reserved words.
pub(crate) fn sqlserver_profile() -> FingerprintProfile {
    FingerprintProfile {
        texts: sqlserver_texts,
        is_reserved,
        folds_in_context: |_, _| false,
    }
}
