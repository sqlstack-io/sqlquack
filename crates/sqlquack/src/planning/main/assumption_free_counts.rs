//! `assumption_free_counts`: unit-testable and strict files relying on no named assumption.

use std::collections::BTreeSet;

use crate::planning::_helpers::outcomes::{assumption_free, outcomes};
use crate::planning::models::PlanInput;

/// The unit-testable and strict files once those relying on an assumption, by `relied`, drop out.
pub(crate) fn assumption_free_counts(
    input: &PlanInput,
    relied: &[BTreeSet<String>],
) -> (usize, usize) {
    let current = outcomes(input, &BTreeSet::new());
    let assuming: Vec<bool> = relied.iter().map(|ids| !ids.is_empty()).collect();
    let free = assumption_free(input, &current, &assuming);
    let count = |flags: &[bool]| flags.iter().filter(|flag| **flag).count();
    (count(&free.unit), count(&free.strict))
}
