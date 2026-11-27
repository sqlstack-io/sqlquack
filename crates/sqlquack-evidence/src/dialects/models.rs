//! The shared evidence profile.

use crate::dialects::types::EvidenceProfile;

/// Cases whose input rows are a `UNION ALL` of typed `SELECT`s, and results stored as typed Arrow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SharedEvidence;

impl EvidenceProfile for SharedEvidence {}
