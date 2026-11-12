//! `diff_snapshots`: what changed between two snapshots, regressions apart from gains.

use std::collections::{BTreeMap, BTreeSet};

use crate::check::models::FormState;
use crate::report::_helpers::obligation_order::sorted_obligations;
use crate::report::models::{ReceiptDisagreements, Snapshot, SnapshotDiff};

/// A state the checker claims: verified, or excluded by a current review.
fn claimed(state: Option<&FormState>) -> bool {
    matches!(state, Some(FormState::Verified | FormState::Excluded))
}

/// Every case id one side of `disagreements` lists: each form's keys under the form.
fn listed(by_form: &BTreeMap<String, BTreeSet<String>>) -> BTreeSet<String> {
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for (form, keys) in by_form {
        for key in keys {
            ids.insert(format!("{form}.{key}"));
        }
    }
    ids
}

/// `head` against `base`, and its receipt disagreements against those listed now and on the target.
pub fn diff_snapshots(
    base: &Snapshot,
    head: &Snapshot,
    disagreements: &ReceiptDisagreements,
    target_disagreements: Option<&ReceiptDisagreements>,
) -> SnapshotDiff {
    let accepted: BTreeSet<String> = listed(&disagreements.syntax_errors_accepted);
    let refused: BTreeSet<String> = listed(&disagreements.parsed_cases_refused);
    let other: &BTreeSet<String> = &disagreements.other_receipts_contradicted;
    let added: Vec<String> = match target_disagreements {
        Some(target) => {
            let mut target_ids: BTreeSet<String> = listed(&target.syntax_errors_accepted);
            target_ids.extend(listed(&target.parsed_cases_refused));
            target_ids.extend(target.other_receipts_contradicted.iter().cloned());
            let mut ids: BTreeSet<String> = accepted.union(&refused).cloned().collect();
            ids.extend(other.iter().cloned());
            ids.difference(&target_ids).cloned().collect()
        }
        None => Vec::new(),
    };
    let paths: BTreeSet<&String> = base
        .catalogue_files
        .keys()
        .chain(head.catalogue_files.keys())
        .collect();
    let forms: BTreeSet<&String> = base.forms.keys().chain(head.forms.keys()).collect();
    let in_base = |id: &String| base.forms.get(id);
    let in_head = |id: &String| head.forms.get(id);
    let flipped: Vec<String> = base
        .syntax_error_cases_rejected
        .intersection(&head.syntax_error_cases_accepted)
        .cloned()
        .collect();
    let empty: BTreeSet<String> = BTreeSet::new();
    let head_contradicted: &BTreeSet<String> =
        head.receipt_contradictions.as_ref().unwrap_or(&empty);
    let judged_other: &BTreeSet<String> = match head.receipt_contradictions {
        Some(_) => other,
        None => &empty,
    };
    let head_other: BTreeSet<String> = head_contradicted
        .iter()
        .filter(|id| {
            !head.syntax_error_cases_accepted.contains(*id) && !head.refused_cases.contains(*id)
        })
        .cloned()
        .collect();
    let new_contradictions: Vec<String> = match &base.receipt_contradictions {
        Some(base_contradicted) => head_contradicted
            .difference(base_contradicted)
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    SnapshotDiff {
        receipts_contradicted: new_contradictions
            .iter()
            .filter(|id| !head.documented_cases.contains(*id))
            .cloned()
            .collect(),
        documented_receipts_contradicted: new_contradictions
            .iter()
            .filter(|id| head.documented_cases.contains(*id))
            .cloned()
            .collect(),
        receipts_still_contradicted: head_contradicted
            .iter()
            .filter(|id| !new_contradictions.contains(id))
            .cloned()
            .collect(),
        unjudged_receipts: head.unjudged_receipts.iter().flatten().cloned().collect(),
        catalogue_files_changed: paths
            .into_iter()
            .filter(|path| base.catalogue_files.get(*path) != head.catalogue_files.get(*path))
            .cloned()
            .collect(),
        forms_added: forms
            .iter()
            .filter(|id| in_base(id).is_none())
            .map(|id| (*id).clone())
            .collect(),
        forms_removed: forms
            .iter()
            .filter(|id| in_head(id).is_none())
            .map(|id| (*id).clone())
            .collect(),
        forms_lost: forms
            .iter()
            .filter(|id| claimed(in_base(id)) && in_base(id) != in_head(id))
            .map(|id| (*id).clone())
            .collect(),
        forms_gained: forms
            .iter()
            .filter(|id| claimed(in_head(id)) && in_base(id) != in_head(id))
            .map(|id| (*id).clone())
            .collect(),
        obligations_lost: sorted_obligations(
            base.satisfied_obligations
                .difference(&head.satisfied_obligations),
        ),
        obligations_gained: sorted_obligations(
            head.satisfied_obligations
                .difference(&base.satisfied_obligations),
        ),
        parsed_cases_refused: base
            .parsed_cases
            .intersection(&head.refused_cases)
            .cloned()
            .collect(),
        syntax_error_cases_accepted: flipped
            .iter()
            .filter(|id| !head.documented_cases.contains(*id))
            .cloned()
            .collect(),
        documented_cases_accepted: flipped
            .into_iter()
            .filter(|id| head.documented_cases.contains(id))
            .collect(),
        unlisted_syntax_errors_accepted: head
            .syntax_error_cases_accepted
            .difference(&accepted)
            .cloned()
            .collect(),
        unlisted_parsed_cases_refused: head.refused_cases.difference(&refused).cloned().collect(),
        unlisted_receipts_contradicted: head_other.difference(other).cloned().collect(),
        disagreements_resolved: accepted
            .difference(&head.syntax_error_cases_accepted)
            .chain(refused.difference(&head.refused_cases))
            .chain(judged_other.difference(&head_other))
            .cloned()
            .collect(),
        disagreements_added: added,
    }
}
