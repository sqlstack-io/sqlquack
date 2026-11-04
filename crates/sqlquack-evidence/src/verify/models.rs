//! Verification outcomes.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use sqlquack_compare::comparison::models::Mismatch;
use sqlquack_core::failures::models::ErrorKind;
use sqlquack_core::parsing::models::SourceLocation;
use sqlquack_core::provenance::models::Provenance;
use sqlquack_policy::policy::models::Policy;

use crate::cases::models::LoadedCase;
use crate::replay::models::PythonReplay;
use crate::results::models::SessionDefaults;
use crate::verify::types::{CaseConverter, FailureModes};

/// What a converter did with a case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conversion {
    /// DuckDB SQL to replay, and what the conversion relied on.
    Converted {
        /// The DuckDB SQL.
        sql: String,
        /// Statements run first, such as the macros of the case's project functions.
        setup: Vec<String>,
        /// The equivalences, rules, building blocks and typing functions used.
        provenance: Provenance,
        /// Where each raise ID in the SQL comes from in the case's source SQL.
        raises: BTreeMap<String, Vec<SourceLocation>>,
        /// The Python functions the SQL calls, registered before it runs.
        python: Option<Box<PythonReplay>>,
    },
    /// The converter refused.
    Refused {
        /// Why, one line per refusal.
        reasons: Vec<String>,
        /// The refused construct keys, sorted; `statement` when a refusal names no construct.
        constructs: Vec<String>,
    },
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
        /// The refused construct keys, sorted.
        constructs: Vec<String>,
    },
    /// The replay matched the recorded result, or an equivalence's spellings recorded one result.
    Passed,
    /// As the case expects, Snowflake returned rows and DuckDB raised, under a declared divergence.
    Diverged {
        /// The divergence id.
        divergence: String,
        /// DuckDB's error.
        error: String,
        /// Why the error is the divergence's: the guard's tag, or the native error pattern.
        matched: String,
        /// Whether each row, replayed alone, matched its recorded row or raised the divergence.
        rows_proven: bool,
    },
    /// A typing case is recorded: its column types and nullability are evidence for analysis.
    Typed,
    /// The recording cannot show whether the replay is right, so the case neither passes nor fails.
    Unproven {
        /// Why.
        reason: String,
    },
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
    /// What the conversion relied on, for converted cases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    /// For a passed case where Snowflake and DuckDB both raised, how the two errors compare.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_check: Option<ErrorCheck>,
    /// For an error check, Snowflake's message: the stored one, else a re-recording's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snowflake_message: Option<String>,
}

/// How DuckDB's error compares with the recorded Snowflake error, when both raised.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ErrorCheck {
    /// The same kind, and a rule the conversion used declares the Snowflake error.
    Agrees {
        /// The kind.
        kind: ErrorKind,
    },
    /// DuckDB raised a known divergence's guard: a conservative raise where Snowflake raised too.
    DivergenceRaise {
        /// The divergence id.
        divergence: String,
    },
    /// The taxonomy does not know the recorded Snowflake error.
    UnknownSnowflakeError {
        /// Its number.
        errno: Option<i64>,
        /// Its SQLSTATE.
        sqlstate: Option<String>,
    },
    /// A building block tagged another Snowflake error than the recorded one.
    ErrnoMismatch {
        /// The recorded number.
        recorded: i64,
        /// The tagged number.
        tagged: i64,
    },
    /// DuckDB's error is another kind than Snowflake's, or of no known kind.
    KindMismatch {
        /// Snowflake's kind.
        snowflake: ErrorKind,
        /// DuckDB's kind, when known.
        duckdb: Option<ErrorKind>,
        /// DuckDB's error.
        error: String,
    },
    /// No rule the conversion used declares the recorded Snowflake error.
    Undeclared {
        /// Its number.
        errno: i64,
        /// Its SQLSTATE.
        sqlstate: String,
    },
}

/// Something the converter relies on that recorded cases must verify.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Claim {
    /// A recorded spelling pair: two spellings with identical recorded results.
    Equivalence(String),
    /// A conversion rule.
    Rule(String),
}

/// Whether recorded cases verify a claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ClaimVerdict {
    /// The claim.
    pub claim: Claim,
    /// Passed cases that rely on it.
    pub passed_cases: usize,
}

/// What verification runs against.
#[derive(Clone, Copy, Debug)]
pub struct VerifyInputs<'a> {
    /// The loaded cases.
    pub cases: &'a [LoadedCase],
    /// The recorded results directory.
    pub results: &'a Path,
    /// The policy.
    pub policy: &'a Policy,
    /// The converter under test.
    pub converter: &'a dyn CaseConverter,
    /// Everything the converter relies on that cases must verify.
    pub claims: &'a [Claim],
    /// The failure modes each rule declares.
    pub failure_modes: &'a FailureModes,
    /// Re-recorded results, read for the messages of errors recorded before messages were kept.
    pub audit_results: Option<&'a Path>,
    /// The account's recorded session defaults, vouching for results recorded before pinning.
    pub session_defaults: Option<&'a SessionDefaults>,
}

/// Verification of every case.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VerifyReport {
    /// One verdict per case; canonical spellings of equivalences are judged with their pair.
    pub cases: Vec<CaseVerdict>,
    /// One verdict per claim.
    pub claims: Vec<ClaimVerdict>,
    /// Everything the passed conversions relied on, merged.
    pub verified: Provenance,
    /// Recorded results no current case names; kept, because results are append-only.
    pub unreferenced_results: Vec<String>,
    /// Results recorded before pinning while no account defaults are recorded to vouch for them.
    pub unproven_sessions: usize,
}

/// How a replay compares with a recording under the strict comparator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordingMatch {
    /// Every name, declared type and value matches.
    Matches,
    /// The recording kept declared nanosecond columns in microseconds; the replay matches there.
    MatchesAtMicroseconds,
    /// The first difference.
    Differs(Mismatch),
}
