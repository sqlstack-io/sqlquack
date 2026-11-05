//! One case's result: its receipts against the current parser verdict.

use std::collections::{BTreeMap, BTreeSet};

use crate::check::models::{CaseResult, LocalVerdict, Reason};
use crate::formats::models::{AccountSetting, Case, Expect, Phase, Receipt};

/// A syntax receipt bound to exactly this case's SQL, setup, session and capture, with a query id.
fn admissible(case: &Case, receipt: &Receipt) -> bool {
    receipt.measure.is_none()
        && receipt.capture_method == case.capture
        && receipt.sql_sha256 == case.sql_sha256
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

/// Whether the recorded phase is the syntax verdict the case expects; stages are never compared.
fn expected(case: &Case, phase: Phase) -> bool {
    match case.expect {
        None | Some(Expect::Terminates) => true,
        Some(Expect::Reject) => phase == Phase::SyntaxError,
        Some(Expect::Accept) => phase != Phase::SyntaxError,
        Some(Expect::Observe) => false,
    }
}

/// Whether the local verdict agrees with the phase (any clean one for `terminates`), syntax only.
fn agreement(phase: Phase, local: &LocalVerdict, terminates: bool) -> Result<bool, Reason> {
    match (phase, local) {
        (_, LocalVerdict::Failed { .. }) => Err(Reason::LocalParseFailure),
        _ if terminates => Ok(false),
        (Phase::SyntaxError, LocalVerdict::Rejected) => Ok(false),
        (Phase::SyntaxError, LocalVerdict::Accepted { .. }) | (_, LocalVerdict::Rejected) => {
            Err(Reason::Disagreement)
        }
        (_, LocalVerdict::Accepted { opaque: true, .. }) => Err(Reason::OpaqueFallback),
        (Phase::Accepted, LocalVerdict::Accepted { opaque: false, .. }) => Ok(false),
        (_, LocalVerdict::Accepted { opaque: false, .. }) => Ok(true),
    }
}

/// The case's result against its receipts and the current parser's verdict.
pub(crate) fn case_result(case: &Case, receipts: &[&Receipt], local: LocalVerdict) -> CaseResult {
    let (phase, reason, semantic_only): (Option<Phase>, Option<Reason>, bool) =
        match receipt_phase(case, receipts) {
            _ if case.expect == Some(Expect::Observe) => {
                (None, Some(Reason::AwaitingObservation), false)
            }
            Err(reason) => (None, Some(reason), false),
            Ok(phase) if !expected(case, phase) => {
                (Some(phase), Some(Reason::UnexpectedOutcome), false)
            }
            Ok(phase) => match agreement(phase, &local, case.expect == Some(Expect::Terminates)) {
                Ok(semantic_only) => (Some(phase), None, semantic_only),
                Err(reason) => (Some(phase), Some(reason), false),
            },
        };
    CaseResult {
        case_id: case.case_id.clone(),
        sql_sha256: case.sql_sha256.clone(),
        phase,
        local,
        reason,
        semantic_only,
        account_state: receipts
            .iter()
            .filter(|receipt| admissible(case, receipt))
            .flat_map(|receipt| receipt.account_state.iter().flatten())
            .map(|(name, setting)| (name.clone(), setting.clone()))
            .collect::<BTreeMap<String, AccountSetting>>(),
    }
}
