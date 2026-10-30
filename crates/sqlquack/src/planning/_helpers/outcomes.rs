//! Which files convert when some constructs are supported.

use std::collections::BTreeSet;

use crate::census::models::RootKey;
use crate::planning::models::{Outcomes, PlanInput};

/// The unit-testable and strict files once every root in `supported` is resolved.
pub(crate) fn outcomes(input: &PlanInput, supported: &BTreeSet<String>) -> Outcomes {
    let mut unit: Vec<bool> = Vec::with_capacity(input.own.len());
    for (own, parsed) in input.own.iter().zip(&input.parsed) {
        unit.push(*parsed && remaining(own, supported).is_empty());
    }
    let unit = narrowed(unit, &input.needs);
    let strict = narrowed(unit.clone(), &input.reads);
    Outcomes { unit, strict }
}

/// `own` without the roots `supported` resolves.
pub(crate) fn remaining(
    own: &BTreeSet<RootKey>,
    supported: &BTreeSet<String>,
) -> BTreeSet<RootKey> {
    let mut left: BTreeSet<RootKey> = BTreeSet::new();
    for root in own {
        if !supported.contains(&root.construct) {
            left.insert(root.clone());
        }
    }
    left
}

/// `flags` with each flag cleared whose file depends on a file without it, to a fixed point.
fn narrowed(mut flags: Vec<bool>, depends: &[BTreeSet<usize>]) -> Vec<bool> {
    let mut changed = true;
    while changed {
        changed = false;
        for index in 0..flags.len() {
            if flags[index] && depends[index].iter().any(|other| !flags[*other]) {
                flags[index] = false;
                changed = true;
            }
        }
    }
    flags
}
