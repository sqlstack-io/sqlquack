//! `check_equivalence_cases`: every spelling pair has recorded cases, and every pair file is known.

use std::collections::BTreeSet;

use sqlquack_evidence::cases::models::{CaseRole, LoadedCase};
use sqlquack_rules::conversion::constants::SPELLING_PAIRS;

/// Pair cases naming an undeclared pair, and declared pairs without cases.
pub fn check_equivalence_cases(cases: &[LoadedCase]) -> Vec<String> {
    let declared: BTreeSet<&str> = SPELLING_PAIRS.iter().map(|pair| pair.id).collect();
    let mut problems: Vec<String> = Vec::new();
    let mut covered: BTreeSet<&str> = BTreeSet::new();
    for case in cases {
        let CaseRole::EquivalenceSource { equivalence, .. } = &case.role else {
            continue;
        };
        covered.insert(equivalence.as_str());
        if !declared.contains(equivalence.as_str()) {
            problems.push(format!(
                "{}: declares the unknown spelling pair {equivalence}",
                case.reference
            ));
        }
    }
    for pair in SPELLING_PAIRS {
        if !covered.contains(pair.id) {
            problems.push(format!("spelling pair {} has no cases", pair.id));
        }
    }
    problems
}
