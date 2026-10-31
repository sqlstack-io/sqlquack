//! Greedy steps: each supports the few constructs that gain the most files together.

use std::collections::{BTreeMap, BTreeSet};

use crate::census::constants::{PLAN_SET_SIZE, PLAN_STEPS, PLANNABLE_CATEGORIES};
use crate::census::models::{PlanStep, RootKey};
use crate::planning::_helpers::outcomes::{outcomes, remaining};
use crate::planning::models::{Outcomes, PlanInput};

/// Up to `PLAN_STEPS` steps from `supported`, each adding the candidate set gaining the most files.
pub(crate) fn greedy_steps(input: &PlanInput, supported: &BTreeSet<String>) -> Vec<PlanStep> {
    let mut supported = supported.clone();
    let mut current = outcomes(input, &supported);
    let mut steps: Vec<PlanStep> = Vec::new();
    for _ in 0..PLAN_STEPS {
        let mut best: Option<(usize, BTreeSet<RootKey>, Outcomes)> = None;
        for candidate in candidates(input, &supported, &current) {
            let mut trial = supported.clone();
            trial.extend(candidate.iter().map(|root| root.construct.clone()));
            let after = outcomes(input, &trial);
            let gain = converted(&after) - converted(&current);
            let better = match &best {
                None => gain > 0,
                Some((best_gain, best_set, _)) => {
                    (gain, std::cmp::Reverse(candidate.len()))
                        > (*best_gain, std::cmp::Reverse(best_set.len()))
                }
            };
            if better {
                best = Some((gain, candidate, after));
            }
        }
        let Some((_, roots, after)) = best else {
            break;
        };
        supported.extend(roots.iter().map(|root| root.construct.clone()));
        steps.push(PlanStep {
            roots: roots.into_iter().collect(),
            unit_testable_upper_bound: count(&after.unit),
            strict_upper_bound: count(&after.strict),
            unit_testable_possibly_incomplete: possibly_incomplete(&after, &input.incomplete),
        });
        current = after;
    }
    steps
}

/// Of the unit-testable files, those that may hold refusals collecting did not reach.
pub(crate) fn possibly_incomplete(outcomes: &Outcomes, incomplete: &[bool]) -> usize {
    outcomes
        .unit
        .iter()
        .zip(incomplete)
        .filter(|(unit, incomplete)| **unit && **incomplete)
        .count()
}

/// The unit-testable and strict files together.
fn converted(outcomes: &Outcomes) -> usize {
    count(&outcomes.unit) + count(&outcomes.strict)
}

fn count(flags: &[bool]) -> usize {
    flags.iter().filter(|flag| **flag).count()
}

/// The small sets of unsupported roots that would make some file unit-testable or strict.
fn candidates(
    input: &PlanInput,
    supported: &BTreeSet<String>,
    current: &Outcomes,
) -> BTreeSet<BTreeSet<RootKey>> {
    let left: Vec<BTreeSet<RootKey>> = input
        .own
        .iter()
        .map(|own| remaining(own, supported))
        .collect();
    let mut sets: BTreeSet<BTreeSet<RootKey>> = BTreeSet::new();
    for index in 0..left.len() {
        for (done, depends) in [
            (&current.unit, &input.needs),
            (&current.strict, &input.reads),
        ] {
            if done[index] || !input.parsed[index] {
                continue;
            }
            let set = closure_roots(index, depends, &left);
            if !set.is_empty() && set.len() <= PLAN_SET_SIZE && set.iter().all(plannable) {
                sets.insert(set);
            }
        }
    }
    sets
}

/// The roots left in `start` and every file it depends on, transitively.
fn closure_roots(
    start: usize,
    depends: &[BTreeSet<usize>],
    left: &[BTreeSet<RootKey>],
) -> BTreeSet<RootKey> {
    let mut seen: BTreeMap<usize, ()> = BTreeMap::new();
    let mut pending: Vec<usize> = vec![start];
    let mut roots: BTreeSet<RootKey> = BTreeSet::new();
    while let Some(index) = pending.pop() {
        if seen.insert(index, ()).is_none() {
            roots.extend(left[index].iter().cloned());
            pending.extend(depends[index].iter().copied());
        }
    }
    roots
}

/// Whether a root is a missing rule or a rule's type condition, which a rule could resolve.
fn plannable(root: &RootKey) -> bool {
    PLANNABLE_CATEGORIES
        .iter()
        .any(|category| root.category == category.label())
}
