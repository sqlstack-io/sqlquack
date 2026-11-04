//! One case's outcome: its receipts against the current parser verdict.

use std::collections::BTreeSet;

use crate::check::models::{LocalVerdict, Reason};
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

/// Whether the case agrees, and whether only syntax agrees (Snowflake stopped after parsing).
pub(crate) fn case_outcome(
    case: &Case,
    receipts: &[&Receipt],
    local: LocalVerdict,
) -> Result<bool, Reason> {
    if receipts.is_empty() {
        return Err(Reason::MissingReceipt);
    }
    let phases: BTreeSet<String> = receipts
        .iter()
        .filter(|receipt| admissible(case, receipt))
        .map(|receipt| format!("{:?}", receipt.phase))
        .collect();
    let phase: Phase = match receipts
        .iter()
        .find(|receipt| admissible(case, receipt))
        .map(|receipt| receipt.phase)
    {
        None => return Err(Reason::StaleReceipt),
        Some(_) if phases.len() > 1 => return Err(Reason::AmbiguousReceipts),
        Some(phase) => phase,
    };
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
