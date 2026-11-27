//! The shared evidence profile, and why a build cannot read a warehouse's evidence.

use crate::dialects::models::SharedEvidence;

/// The shared profile, for warehouses that need no profile of their own.
pub const SHARED_EVIDENCE: SharedEvidence = SharedEvidence;

/// Why evidence recorded on a warehouse whose feature is off cannot be read.
pub const NOT_COMPILED: &str = "this build reads no evidence recorded on";
