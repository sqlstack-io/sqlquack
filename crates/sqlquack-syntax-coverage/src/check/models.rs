//! What the checker decides for each obligation, form and page.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::formats::models::{Form, Obligation, ObligationKind, Phase};

/// The current parser's verdict on one case's SQL.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum LocalVerdict {
    /// Parsed; `opaque` when a statement or any node in it is a Command or Raw fallback.
    Accepted {
        opaque: bool,
    },
    Rejected,
}

/// Why an obligation or a form is not satisfied; every failure names one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// The form declares no required obligation.
    NoObligations,
    /// No case cites the obligation.
    MissingCase,
    /// A citing case has no receipt.
    MissingReceipt,
    /// A citing case has receipts, none bound to its SQL, setup and session hashes and query.
    StaleReceipt,
    /// A citing case has admissible receipts of different phases.
    AmbiguousReceipts,
    /// Snowflake's recorded phase contradicts what the case expects, so it is no evidence.
    UnexpectedOutcome,
    /// The current parser's verdict differs from the receipt's phase.
    Disagreement,
    /// The current parser accepts a case only through a Command or Raw fallback.
    OpaqueFallback,
    /// No review names the form, by its family or in its `forms` list.
    Unreviewed,
    /// No approved review matches the current catalogue, obligations and parser tree.
    StaleReview,
    /// Reserved for a capture hold; no current form is held.
    BlockedByHold,
}

/// One case's result: its receipts against the current parser.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
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
}

/// What every form of one family is judged against besides its own obligations.
#[derive(Debug)]
pub(crate) struct FamilyContext<'a> {
    pub(crate) joined: &'a Joined<'a>,
    /// Why the form lacks a current review of its family, if it does.
    pub(crate) review: Option<Reason>,
    /// Form ids excluded by current approved reviews.
    pub(crate) excluded: &'a BTreeSet<String>,
}
