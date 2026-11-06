//! Whether one statement's or form's change from the baseline regresses.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use crate::results::models::{ObligationState, ResultState, ResultsDiff, StatementOutcome};

/// Whether a change from the old value or none to the new or none regresses.
pub(crate) type Regresses<T> = fn(Option<T>, Option<T>) -> bool;

/// A statement drops out of matching, or newly differs.
pub(crate) fn statement_regresses(
    base: Option<StatementOutcome>,
    head: Option<StatementOutcome>,
) -> bool {
    let was_matching = base.is_some_and(StatementOutcome::matches);
    let differs = head == Some(StatementOutcome::Differs) && base != head;
    (was_matching && !head.is_some_and(StatementOutcome::matches)) || differs
}

/// A form drops out of matching, or newly differs.
pub(crate) fn form_regresses(base: Option<ResultState>, head: Option<ResultState>) -> bool {
    let was_matching = base == Some(ResultState::Matching);
    let differs = head == Some(ResultState::Differs) && base != head;
    (was_matching && head != Some(ResultState::Matching)) || differs
}

/// An obligation stops being met, or a met one is dropped.
pub(crate) fn obligation_regresses(
    base: Option<ObligationState>,
    head: Option<ObligationState>,
) -> bool {
    base == Some(ObligationState::Met) && head != Some(ObligationState::Met)
}

/// Each changed key of one kind, as a regression or another change.
pub(crate) fn compare<T: Copy + PartialEq + Debug>(
    label: &str,
    (base, head): (&BTreeMap<String, T>, &BTreeMap<String, T>),
    regresses: Regresses<T>,
) -> ResultsDiff {
    let mut diff = ResultsDiff::default();
    let keys: BTreeSet<&String> = base.keys().chain(head.keys()).collect();
    for key in keys {
        let (before, after) = (base.get(key).copied(), head.get(key).copied());
        if before == after {
            continue;
        }
        let line = format!("{label} {key}: {before:?} -> {after:?}");
        match regresses(before, after) {
            true => diff.regressions.push(line),
            false => diff.changes.push(line),
        }
    }
    diff
}
