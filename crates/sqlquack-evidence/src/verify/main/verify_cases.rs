//! `verify_cases`: every case's verdict against its recorded result, and every claim's support.

use std::collections::BTreeSet;
use std::path::Path;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use sqlquack_diagnostics::provenance::main::merge_provenance::merge_provenance;

use crate::cases::models::{CaseRole, LoadedCase};
use crate::errors::EvidenceError;
use crate::results::main::recorded_hashes::recorded_hashes;
use crate::results::models::SessionDefaults;
use crate::verify::_helpers::claims::claim_verdicts;
use crate::verify::_helpers::judge::judge;
use crate::verify::_helpers::messages::recorded_message;
use crate::verify::_helpers::replay_cache::replay_context;
use crate::verify::models::{CaseVerdict, ReplayCache, Verdict, VerifyInputs, VerifyReport};
use crate::verify::types::{CaseConverter, FailureModes};
use sqlquack_policy::policy::models::Policy;

/// Replays recorded cases, checks equivalence pairs, and counts the passed cases behind claims.
pub fn verify_cases(inputs: VerifyInputs<'_>) -> Result<VerifyReport, EvidenceError> {
    let VerifyInputs {
        cases,
        results,
        policy,
        converter,
        claims,
        failure_modes,
        audit_results,
        session_defaults,
        replay_cache,
    } = inputs;
    let context = replay_cache.map(|cache| replay_context(&cache.scope, policy, failure_modes));
    let cache = replay_cache.zip(context.as_deref());
    let referenced: BTreeSet<&str> = cases.iter().map(|case| case.hash.as_str()).collect();
    let unreferenced_results: Vec<String> = recorded_hashes(results)?
        .into_iter()
        .filter(|hash| !referenced.contains(hash.as_str()))
        .collect();
    let judged: Vec<(CaseVerdict, bool)> = cases
        .par_iter()
        .filter(|case| !matches!(case.role, CaseRole::EquivalenceCanonical { .. }))
        .map(|case| {
            judged_case(
                case,
                (results, audit_results, session_defaults),
                (policy, cache),
                (converter, failure_modes),
            )
        })
        .collect::<Result<_, EvidenceError>>()?;
    let unproven_sessions: usize = judged.iter().filter(|(_, unproven)| *unproven).count();
    let verdicts: Vec<CaseVerdict> = judged.into_iter().map(|(verdict, _)| verdict).collect();
    let verified = merge_provenance(
        verdicts
            .iter()
            .filter(|verdict| verdict.verdict == Verdict::Passed)
            .filter_map(|verdict| verdict.provenance.as_ref()),
    );
    Ok(VerifyReport {
        claims: claim_verdicts(claims, cases, &verdicts),
        verified,
        cases: verdicts,
        unreferenced_results,
        unproven_sessions,
    })
}

/// One case's verdict with Snowflake's message, and whether its result's session is unproven.
fn judged_case(
    case: &LoadedCase,
    (results, audit_results, session_defaults): (&Path, Option<&Path>, Option<&SessionDefaults>),
    (policy, cache): (&Policy, Option<(&ReplayCache, &str)>),
    checks: (&dyn CaseConverter, &FailureModes),
) -> Result<(CaseVerdict, bool), EvidenceError> {
    let ((verdict, provenance, error_check), unproven) =
        judge(case, (results, session_defaults), (policy, cache), checks);
    let snowflake_message = match error_check {
        Some(_) => recorded_message(&case.hash, results, audit_results)?,
        None => None,
    };
    let verdict = CaseVerdict {
        reference: case.reference.clone(),
        hash: case.hash.clone(),
        verdict,
        provenance,
        error_check,
        snowflake_message,
    };
    Ok((verdict, unproven))
}
