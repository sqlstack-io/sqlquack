//! Counting the passed cases behind each claim.

use std::collections::BTreeMap;

use crate::cases::models::{CaseRole, LoadedCase};
use crate::verify::models::{CaseVerdict, Claim, ClaimVerdict, Verdict};

pub(crate) fn claim_verdicts(
    claims: &[Claim],
    cases: &[LoadedCase],
    verdicts: &[CaseVerdict],
) -> Vec<ClaimVerdict> {
    let roles: BTreeMap<&str, &CaseRole> = cases
        .iter()
        .map(|case| (case.reference.as_str(), &case.role))
        .collect();
    let mut passed: BTreeMap<Claim, usize> = BTreeMap::new();
    for verdict in verdicts
        .iter()
        .filter(|verdict| verdict.verdict == Verdict::Passed)
    {
        if let Some(CaseRole::EquivalenceSource { equivalence, .. }) =
            roles.get(verdict.reference.as_str())
        {
            *passed
                .entry(Claim::Equivalence(equivalence.clone()))
                .or_default() += 1;
        }
        for rule in verdict
            .provenance
            .iter()
            .flat_map(|provenance| &provenance.rules)
        {
            *passed.entry(Claim::Rule(rule.clone())).or_default() += 1;
        }
    }
    let mut sorted: Vec<Claim> = claims.to_vec();
    sorted.sort();
    sorted.dedup();
    sorted
        .into_iter()
        .map(|claim| ClaimVerdict {
            passed_cases: passed.get(&claim).copied().unwrap_or_default(),
            claim,
        })
        .collect()
}
