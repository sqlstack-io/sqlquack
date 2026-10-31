//! What still blocks files once constructs are assumed supported.

use std::collections::{BTreeMap, BTreeSet};

use crate::census::constants::NEXT_ROOTS;
use crate::census::models::{NextRoot, RootKey};
use crate::planning::_helpers::outcomes::remaining;
use crate::planning::models::{Outcomes, PlanInput};

/// The roots still blocking files not unit-testable `after` the assumptions, most files first.
pub(crate) fn next_roots(
    input: &PlanInput,
    assumed: &BTreeSet<String>,
    after: &Outcomes,
) -> Vec<NextRoot> {
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
    next
}

/// The assumed constructs no collected root names.
pub(crate) fn unmatched(input: &PlanInput, assumed: &BTreeSet<String>) -> Vec<String> {
    let collected: BTreeSet<String> = input
        .own
        .iter()
        .flatten()
        .map(|root| root.construct.clone())
        .collect();
    assumed.difference(&collected).cloned().collect()
}
