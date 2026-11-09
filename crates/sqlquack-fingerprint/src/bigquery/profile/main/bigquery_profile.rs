//! `bigquery_profile`: how BigQuery SQL is fingerprinted.

use sqlquack_parse::bigquery::keywords::main::is_reserved::is_reserved;

use crate::bigquery::profile::_helpers::texts::bigquery_texts;
use crate::dialects::models::FingerprintProfile;

/// BigQuery SQL's fingerprint profile: its tokens read by BigQuery's own scanner, with its parser's reserved words.
pub(crate) fn bigquery_profile() -> FingerprintProfile {
    FingerprintProfile {
        texts: bigquery_texts,
        is_reserved,
    }
}
