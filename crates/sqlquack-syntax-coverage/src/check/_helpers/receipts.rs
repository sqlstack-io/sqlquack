//! One case's result: its receipts against the current parser verdict.

use std::collections::BTreeSet;

use crate::check::models::{CaseResult, LocalVerdict, Reason};
use crate::formats::models::{Case, Phase, Receipt};

/// A receipt bound to exactly this case's SQL, setup and session, with a query identity.
fn admissible(case: &Case, receipt: &Receipt) -> bool {
    receipt.sql_sha256 == case.sql_sha256
        && receipt.setup_sha256 == case.setup_sha256
        && receipt.session_sha256 == case.session_sha256
        && (receipt
            .query_id
            .as_deref()
            .is_some_and(|query| !query.trim().is_empty())
            || receipt.query_id_sha256.is_some())
}

/// The single phase of the admissible receipts, or why there is none.
fn receipt_phase(case: &Case, receipts: &[&Receipt]) -> Result<Phase, Reason> {
    if receipts.is_empty() {
        return Err(Reason::MissingReceipt);
    }
    let phases: BTreeSet<String> = receipts
        .iter()
        .filter(|receipt| admissible(case, receipt))
        .map(|receipt| format!("{:?}", receipt.phase))
        .collect();
    match receipts.iter().find(|receipt| admissible(case, receipt)) {
        None => Err(Reason::StaleReceipt),
        Some(_) if phases.len() > 1 => Err(Reason::AmbiguousReceipts),
        Some(receipt) => Ok(receipt.phase),
    }
}

/// Whether the local verdict agrees with the phase, and whether only syntax agrees.
fn agreement(phase: Phase, local: LocalVerdict) -> Result<bool, Reason> {
    match (phase, local) {
        (Phase::SyntaxError, LocalVerdict::Rejected) => Ok(false),
        (Phase::SyntaxError, LocalVerdict::Accepted { .. }) | (_, LocalVerdict::Rejected) => {
            Err(Reason::Disagreement)
        }
        (_, LocalVerdict::Accepted { opaque: true }) => Err(Reason::OpaqueFallback),
        (Phase::Accepted, LocalVerdict::Accepted { opaque: false }) => Ok(false),
        (_, LocalVerdict::Accepted { opaque: false }) => Ok(true),
    }
}

/// The case's result against its receipts and the current parser's verdict.
pub(crate) fn case_result(case: &Case, receipts: &[&Receipt], local: LocalVerdict) -> CaseResult {
    let phase: Result<Phase, Reason> = receipt_phase(case, receipts);
    let outcome: Result<bool, Reason> = phase.and_then(|phase| agreement(phase, local));
    CaseResult {
        case_id: case.case_id.clone(),
        phase: phase.ok(),
        local,
        reason: outcome.err(),
        semantic_only: outcome == Ok(true),
    }
}
