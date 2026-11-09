//! `plan_census`: upper bounds once constructs are supported, what blocks next, and a greedy plan.

use std::collections::BTreeSet;

use crate::census::models::PlanReport;
use crate::planning::_helpers::next::{next_roots, unmatched};
use crate::planning::_helpers::outcomes::outcomes;
use crate::planning::_helpers::steps::{greedy_steps, possibly_incomplete};
use crate::planning::models::PlanInput;

/// The plan over `input` with the constructs in `assumed` supported.
pub(crate) fn plan_census(input: &PlanInput, assumed: &BTreeSet<String>) -> PlanReport {
    let after = outcomes(input, assumed);
    PlanReport {
        assumed: assumed.iter().cloned().collect(),
        unit_testable_upper_bound: after.unit.iter().filter(|unit| **unit).count(),
        strict_upper_bound: after.strict.iter().filter(|strict| **strict).count(),
        unit_testable_possibly_incomplete: possibly_incomplete(&after, &input.incomplete),
        unmatched: unmatched(input, assumed),
        next: next_roots(input, assumed, &after),
        steps: greedy_steps(input, assumed),
    }
}
