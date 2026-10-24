//! The pending case file the recorder reads.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sqlquack_policy::policy::models::SessionValue;

/// One case for the recorder to run, written as `<hash>.json`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PendingCase {
    /// The evidence format.
    pub format: u32,
    /// The case hash naming the result to write.
    pub case_hash: String,
    /// `<file>#<id>` of the authored case.
    pub case_reference: String,
    /// The source dialect.
    pub source_dialect: String,
    /// Session parameters to set before running the SQL.
    pub session_parameters: BTreeMap<String, SessionValue>,
    /// The SQL to run.
    pub source_sql: String,
    /// The DuckDB version the case is prepared against.
    pub duckdb_version: String,
}

/// What `write_pending` did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingSummary {
    /// Pending files written.
    pub written: usize,
    /// Cases that already have a result.
    pub recorded: usize,
    /// Stale pending files removed.
    pub removed: usize,
}
