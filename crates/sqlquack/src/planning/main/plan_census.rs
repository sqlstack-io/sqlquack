//! `plan_census`: upper bounds once constructs are supported, what blocks next, and a greedy plan.

use std::collections::{BTreeMap, BTreeSet};

use crate::census::constants::NEXT_ROOTS;
use crate::census::models::{NextRoot, PlanReport, RootKey};
use crate::planning::_helpers::outcomes::{outcomes, remaining};
use crate::planning::_helpers::steps::greedy_steps;
use crate::planning::models::PlanInput;

/// The plan over `input` with the constructs in `assumed` supported.
pub(crate) fn plan_census(input: &PlanInput, assumed: &BTreeSet<String>) -> PlanReport {
    let after = outcomes(input, assumed);
    let mut next: BTreeMap<RootKey, (usize, usize)> = BTreeMap::new();
    for (index, own) in input.own.iter().enumerate() {
        if after.unit[index] {
            continue;
        }
        let left = remaining(own, assumed);
        for root in &left {
            let entry = next.entry(root.clone()).or_default();
            entry.0 += 1;
            entry.1 += usize::from(left.len() == 1);
        }
    }
    let mut next: Vec<NextRoot> = next
        .into_iter()
        .map(|(root, (files, sole))| NextRoot { root, files, sole })
        .collect();
    next.sort_by(|left, right| {
        (right.files, right.sole)
            .cmp(&(left.files, left.sole))
            .then_with(|| left.root.cmp(&right.root))
    });
    next.truncate(NEXT_ROOTS);
    PlanReport {
        assumed: assumed.iter().cloned().collect(),
        unit_testable_upper_bound: after.unit.iter().filter(|unit| **unit).count(),
        strict_upper_bound: after.strict.iter().filter(|strict| **strict).count(),
        possibly_incomplete: after
            .unit
            .iter()
            .zip(&input.incomplete)
            .filter(|(unit, incomplete)| **unit && **incomplete)
            .count(),
        next,
        steps: greedy_steps(input, assumed),
    }
}
