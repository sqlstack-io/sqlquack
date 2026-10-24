//! `verify_cases`: every case's verdict against its recorded result.

use std::collections::BTreeSet;
use std::path::Path;

use sqlquack_policy::policy::models::Policy;

use crate::cases::models::LoadedCase;
use crate::errors::EvidenceError;
use crate::results::main::recorded_hashes::recorded_hashes;
use crate::verify::_helpers::judge::judge;
use crate::verify::models::{CaseVerdict, VerifyReport};
use crate::verify::types::CaseConverter;

/// Converts and replays every recorded case and compares it under the policy.
pub fn verify_cases(
    cases: &[LoadedCase],
    results: &Path,
    policy: &Policy,
    converter: &dyn CaseConverter,
) -> Result<VerifyReport, EvidenceError> {
    let referenced: BTreeSet<&str> = cases.iter().map(|case| case.hash.as_str()).collect();
    let unreferenced_results: Vec<String> = recorded_hashes(results)?
        .into_iter()
        .filter(|hash| !referenced.contains(hash.as_str()))
        .collect();
    Ok(VerifyReport {
        cases: cases
            .iter()
            .map(|case| CaseVerdict {
                reference: case.reference.clone(),
                hash: case.hash.clone(),
                verdict: judge(case, results, policy, converter),
            })
            .collect(),
        unreferenced_results,
    })
}
