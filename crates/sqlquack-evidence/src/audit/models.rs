//! Audit outcomes.

use serde::Serialize;

/// One re-recorded case.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AuditEntry {
    /// The case hash.
    pub hash: String,
    /// `<file>#<id>`, when a current case names the hash.
    pub reference: Option<String>,
    /// The warehouse version of the stored result.
    pub stored_warehouse_version: Option<String>,
    /// The warehouse version of the fresh result.
    pub fresh_warehouse_version: Option<String>,
    /// The comparison.
    #[serde(flatten)]
    pub outcome: AuditOutcome,
}

/// How a fresh recording compares with the stored one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AuditOutcome {
    /// Identical results.
    Matched,
    /// Different results; with equal warehouse versions this suggests the stored result was altered.
    Differed {
        /// The first difference.
        detail: String,
    },
    /// A result could not be read.
    Invalid {
        /// Why.
        detail: String,
    },
}
