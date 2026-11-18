//! One case's result: its receipts against the current parser verdict.

use std::collections::{BTreeMap, BTreeSet};

use crate::check::models::{CaseResult, LocalVerdict, Reason, SnowflakeSyntax};
use crate::formats::constants::SYNTAX_ERRNO;
use crate::formats::main::current_dialect::current_dialect;
use crate::formats::main::engine_release::engine_release;
use crate::formats::models::{AccountSetting, Case, Expect, NoEffect, Phase, Receipt};

/// A receipt bound to this case's SQL, setup, session and capture; a probe-role receipt binds any setup.
fn admissible(case: &Case, receipt: &Receipt) -> bool {
    let parsed_without_setup: bool =
        receipt.no_effect == Some(NoEffect::ProbeRole) && receipt.shows_parse_without_privileges();
    receipt.measure.is_none()
        && receipt.capture_method == case.capture
        && receipt.sql_sha256 == case.sql_sha256
        && (receipt.setup_sha256 == case.setup_sha256 || parsed_without_setup)
        && receipt.session_sha256 == case.session_sha256
        && (receipt.is_engine()
            || receipt
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
    let admitted: Vec<&Receipt> = receipts
        .iter()
        .copied()
        .filter(|receipt| admissible(case, receipt))
        .collect();
    let admitted: Vec<&Receipt> = match current_dialect().engine_pin.as_deref() {
        Some(pin) => newest_release(pin, admitted)?,
        None => admitted,
    };
    let phases: BTreeSet<String> = admitted
        .iter()
        .map(|receipt| format!("{:?}", receipt.phase))
        .collect();
    match admitted.first() {
        None => Err(Reason::StaleReceipt),
        Some(_) if phases.len() > 1 => Err(Reason::AmbiguousReceipts),
        Some(receipt) => Ok(receipt.phase),
    }
}

/// The receipts of the newest release in the pin's year; a phase another release of that year disagrees with fails.
fn newest_release<'a>(pin: &str, admitted: Vec<&'a Receipt>) -> Result<Vec<&'a Receipt>, Reason> {
    let year: Option<u32> = engine_release(pin).map(|(year, _)| year);
    let dated: Vec<(u32, &'a Receipt)> = admitted
        .into_iter()
        .filter_map(|receipt| {
            let (receipt_year, release) = engine_release(receipt.engine_version.as_deref()?)?;
            (Some(receipt_year) == year).then_some((release, receipt))
        })
        .collect();
    let mut phases: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    for (release, receipt) in &dated {
        phases
            .entry(*release)
            .or_default()
            .insert(format!("{:?}", receipt.phase));
    }
    let Some((newest, newest_phases)) = phases.last_key_value() else {
        return Ok(Vec::new());
    };
    match phases.values().all(|other| other == newest_phases) {
        true => Ok(dated
            .into_iter()
            .filter(|(release, _)| release == newest)
            .map(|(_, receipt)| receipt)
            .collect()),
        false => Err(Reason::ReleaseDisagreement),
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

/// What Snowflake's parser did by the phases the importer recorded for the case's statement; none unless all agree.
pub(crate) fn snowflake_syntax(statement_receipts: &[&Receipt]) -> Option<SnowflakeSyntax> {
    let verdicts: BTreeSet<Option<SnowflakeSyntax>> = statement_receipts
        .iter()
        .filter(|receipt| receipt.measure.is_none())
        .map(|receipt| match (receipt.phase, receipt.errno) {
            _ if receipt.is_engine() => engine_syntax(receipt),
            (Phase::SyntaxError, _) => Some(SnowflakeSyntax::Refused),
            (_, Some(errno)) if errno == SYNTAX_ERRNO => None,
            _ => Some(SnowflakeSyntax::Parsed),
        })
        .collect();
    match verdicts.len() {
        1 => verdicts.into_iter().next().flatten(),
        _ => None,
    }
}

/// What the engine's parser did by a `/2` receipt: refused when a syntax code decided the phase.
fn engine_syntax(receipt: &Receipt) -> Option<SnowflakeSyntax> {
    let syntax_code: bool = receipt
        .engine_code
        .as_ref()
        .is_some_and(|code| current_dialect().syntax_codes.contains(code));
    match (receipt.phase, syntax_code) {
        (Phase::SyntaxError, true) => Some(SnowflakeSyntax::Refused),
        (Phase::SyntaxError, false) => None,
        _ => Some(SnowflakeSyntax::Parsed),
    }
}

/// The case's result against its receipts, its statement's receipts and the parser's verdict.
pub(crate) fn case_result(
    case: &Case,
    receipts: &[&Receipt],
    statement_receipts: &[&Receipt],
    local: LocalVerdict,
) -> CaseResult {
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
        expect: case.expect,
        stage: case.stage,
        phase,
        local,
        reason,
        semantic_only,
        snowflake: snowflake_syntax(statement_receipts),
        account_state: receipts
            .iter()
            .filter(|receipt| admissible(case, receipt))
            .flat_map(|receipt| receipt.account_state.iter().flatten())
            .map(|(name, setting)| (name.clone(), setting.clone()))
            .collect::<BTreeMap<String, AccountSetting>>(),
    }
}
