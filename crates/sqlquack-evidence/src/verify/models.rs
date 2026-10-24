//! Verification outcomes.

use serde::Serialize;

/// What a converter did with a case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conversion {
    /// DuckDB SQL to replay.
    Converted(String),
    /// Refusal reasons.
    Refused(Vec<String>),
}

/// The verdict for one case.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
    /// No result is recorded yet.
    Pending,
    /// The converter refused, which is always acceptable.
    Refused {
        /// Why.
        reasons: Vec<String>,
    },
    /// The replay matched the recorded result.
    Passed,
    /// The replay differed from the recorded result, or the evidence is invalid.
    Failed {
        /// What went wrong.
        reason: String,
    },
}

/// One case's verdict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CaseVerdict {
    /// `<file>#<id>`.
    pub reference: String,
    /// The case hash.
    pub hash: String,
    /// The verdict.
    #[serde(flatten)]
    pub verdict: Verdict,
}

/// Verification of every case.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VerifyReport {
    /// One verdict per case.
    pub cases: Vec<CaseVerdict>,
    /// Recorded results no current case names; kept, because results are append-only.
    pub unreferenced_results: Vec<String>,
}
