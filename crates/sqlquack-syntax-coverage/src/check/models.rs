//! What the checker decides for each obligation, form and page.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::formats::models::{Form, Obligation, ObligationKind, Phase, Review};

/// The current parser's verdict on one case's SQL.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum LocalVerdict {
    /// Parsed; `opaque` when a statement or any node in it is a Command or Raw fallback.
    Accepted {
        opaque: bool,
        /// SHA-256 of the parsed statements as compact JSON: what a review fingerprint covers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tree_sha256: Option<String>,
    },
    Rejected,
    /// The parse did not end in a verdict of its own.
    Failed {
        failure: LocalFailure,
    },
}

/// How a local parse failed to end cleanly (decision D90).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalFailure {
    /// The parser panicked.
    Panic,
    /// The worker died: a stack overflow, an abort or the memory bound.
    Crash,
    /// The parse ran past the time bound.
    Timeout,
    /// A complexity guard refused the input; never a syntax verdict.
    Guard,
}

/// Why an obligation or a form is not satisfied; every failure names one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// The form declares no required obligation.
    NoObligations,
    /// No case cites the obligation.
    MissingCase,
    /// Unverifiable: Snowflake answers the construct only with an internal error (decision D87).
    UnverifiableInternalError,
    /// A citing case has no receipt.
    MissingReceipt,
    /// A citing extreme case awaits Snowflake's answer (`expect = "observe"`).
    AwaitingObservation,
    /// A citing case has receipts, none bound to its SQL, setup and session hashes and query.
    StaleReceipt,
    /// A citing case has admissible receipts of different phases.
    AmbiguousReceipts,
    /// Snowflake's recorded phase contradicts what the case expects, so it is no evidence.
    UnexpectedOutcome,
    /// The local parse panicked, crashed, hit a guard or ran past its time or memory bound.
    LocalParseFailure,
    /// The current parser's verdict differs from the receipt's phase.
    Disagreement,
    /// The current parser accepts a case only through a Command or Raw fallback.
    OpaqueFallback,
    /// No review names the form, by its family or in its `forms` list.
    Unreviewed,
    /// No approved review matches the files and the form's fingerprint, or else the parser tree.
    StaleReview,
    /// Reserved for a capture hold; no current form is held.
    BlockedByHold,
}

/// One case's result: its receipts against the current parser.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    /// The case's declared SQL hash, part of its form's fingerprint.
    pub sql_sha256: String,
    /// The phase its admissible receipts agree on; none when there is no such single phase.
    pub phase: Option<Phase>,
    pub local: LocalVerdict,
    /// Why it does not agree; none when it does.
    pub reason: Option<Reason>,
    /// Agrees, while Snowflake stopped after parsing (semantic, runtime or unsupported phase).
    pub semantic_only: bool,
}

/// One obligation's result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObligationResult {
    pub id: String,
    pub kind: ObligationKind,
    pub target: Option<String>,
    pub required: bool,
    /// The first reason it fails, in `Reason` order; none when satisfied.
    pub reason: Option<Reason>,
    /// Satisfied for syntax, while Snowflake stopped after parsing (semantic or runtime phase).
    pub semantic_only: bool,
    /// Every citing case; the obligation is satisfied only when all of them agree.
    pub cases: Vec<CaseResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormState {
    Verified,
    /// Accepted as excluded by a current approved review.
    Excluded,
    Unverified,
    /// Its grammar left the documentation; kept for its id, never counted.
    Absent,
}

/// One form's result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FormResult {
    pub id: String,
    pub family: String,
    pub source: String,
    /// Pages restating the same grammar; the form still counts once.
    pub also_sources: Vec<String>,
    pub state: FormState,
    /// Every reason it is not verified.
    pub reasons: Vec<Reason>,
    pub required_obligations: usize,
    pub satisfied_obligations: usize,
    /// SHA-256 over the form's cases, sorted by case id: what a review's `fingerprints` binds.
    pub fingerprint: String,
    /// SHA-256 of the form's catalogue entry and its own obligations: what `definitions` binds.
    pub definition_sha256: String,
    pub obligations: Vec<ObligationResult>,
}

/// The whole check: every form, and every problem joining the files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CheckResult {
    /// The parser source tree the verdicts came from; none when it is uncommitted.
    pub parser_tree: Option<String>,
    pub forms: Vec<FormResult>,
    /// Pages without forms that no current approved review excludes.
    pub unexcluded_pages: Vec<String>,
    /// References between files that do not resolve; any one fails the check.
    pub problems: Vec<String>,
}

/// The files joined for checking, with every case already judged.
#[derive(Debug)]
pub(crate) struct Joined<'a> {
    /// Catalogued forms by id.
    pub(crate) forms: BTreeMap<&'a str, &'a Form>,
    /// Obligations by the form they belong to.
    pub(crate) obligations: BTreeMap<&'a str, Vec<&'a Obligation>>,
    /// Case ids by each obligation they cite.
    pub(crate) citing: BTreeMap<&'a str, Vec<&'a str>>,
    /// Each case's result by case id.
    pub(crate) cases: BTreeMap<&'a str, CaseResult>,
    /// Obligations listed as unobservable, which never count as satisfied.
    pub(crate) unobservable: BTreeSet<&'a str>,
}

/// What every form of one family is judged against besides its own obligations.
#[derive(Debug)]
pub(crate) struct FamilyContext<'a> {
    pub(crate) joined: &'a Joined<'a>,
    /// Every review, judged against each form with its fingerprint.
    pub(crate) reviews: &'a [Review],
    /// Each coverage file's SHA-256 by its path under the root.
    pub(crate) hashes: &'a BTreeMap<String, String>,
    /// The parser source tree the verdicts came from; none when it is uncommitted.
    pub(crate) parser_tree: Option<&'a str>,
    /// Form ids excluded by current approved reviews.
    pub(crate) excluded: &'a BTreeSet<String>,
}
