//! `verify_cases`: every case's verdict against its recorded result, and every claim's support.

use std::collections::BTreeSet;

use sqlquack_diagnostics::provenance::main::merge_provenance::merge_provenance;

use crate::cases::models::CaseRole;
use crate::errors::EvidenceError;
use crate::results::main::recorded_hashes::recorded_hashes;
use crate::verify::_helpers::claims::claim_verdicts;
use crate::verify::_helpers::judge::judge;
use crate::verify::_helpers::messages::recorded_message;
use crate::verify::models::{CaseVerdict, Verdict, VerifyInputs, VerifyReport};

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
    } = inputs;
    let referenced: BTreeSet<&str> = cases.iter().map(|case| case.hash.as_str()).collect();
    let unreferenced_results: Vec<String> = recorded_hashes(results)?
        .into_iter()
        .filter(|hash| !referenced.contains(hash.as_str()))
        .collect();
    let mut verdicts: Vec<CaseVerdict> = Vec::new();
    let mut unproven_sessions: usize = 0;
    for case in cases {
        if matches!(case.role, CaseRole::EquivalenceCanonical { .. }) {
            continue;
        }
        let ((verdict, provenance, error_check), unproven) = judge(
            case,
            (results, session_defaults),
            policy,
            (converter, failure_modes),
        );
        unproven_sessions += usize::from(unproven);
        let snowflake_message = match error_check {
            Some(_) => recorded_message(&case.hash, results, audit_results)?,
            None => None,
        };
        verdicts.push(CaseVerdict {
            reference: case.reference.clone(),
            hash: case.hash.clone(),
            verdict,
            provenance,
            error_check,
            snowflake_message,
        });
    }
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
