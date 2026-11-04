//! What the checker decides for each obligation, form and page.

use serde::Serialize;

/// The current parser's verdict on one case's SQL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    /// The current parser's verdict differs from the receipt's phase.
    Disagreement,
    /// The current parser accepts a case only through a Command or Raw fallback.
    OpaqueFallback,
    /// No review names the form's family.
    Unreviewed,
    /// No approved review matches the current catalogue, obligations and parser tree.
    StaleReview,
    /// Reserved for a capture hold; no current form is held.
    BlockedByHold,
}

/// One obligation's result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObligationResult {
    pub id: String,
    pub required: bool,
    /// The first reason it fails, in `Reason` order; none when satisfied.
    pub reason: Option<Reason>,
    /// Satisfied for syntax, while Snowflake stopped after parsing (semantic or runtime phase).
    pub semantic_only: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
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
