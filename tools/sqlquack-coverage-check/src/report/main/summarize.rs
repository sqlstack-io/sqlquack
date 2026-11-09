//! `summarize`: the headline counts of a check, overall and by family.

use std::collections::BTreeMap;

use crate::check::models::{CheckResult, FormState, Reason};
use crate::report::_helpers::tally::{counted, percent};
use crate::report::main::tally_by::tally_by;
use crate::report::models::{FamilySummary, Summary, Tally};

pub fn summarize(check: &CheckResult) -> Summary {
    let mut total: Tally = Tally::default();
    let mut forms_by_reason: BTreeMap<Reason, usize> = BTreeMap::new();
    let mut semantic_only_obligations: usize = 0;
    for form in check
        .forms
        .iter()
        .filter(|form| form.state != FormState::Absent)
    {
        total = counted(total, form);
        if form.state == FormState::Unverified {
            for reason in &form.reasons {
                *forms_by_reason.entry(*reason).or_insert(0) += 1;
            }
        }
        semantic_only_obligations += form.obligations.iter().filter(|o| o.semantic_only).count();
    }
    let families: BTreeMap<String, FamilySummary> = tally_by(check, |form| &form.family)
        .into_iter()
        .map(|(family, tally)| {
            let scope_percent: f64 = percent(tally.forms, total.forms);
            (
                family,
                FamilySummary {
                    tally,
                    scope_percent,
                },
            )
        })
        .collect();
    Summary {
        parser_tree: check.parser_tree.clone(),
        verified_percent: percent(total.verified, total.forms),
        absent_forms: check
            .forms
            .iter()
            .filter(|form| form.state == FormState::Absent)
            .count(),
        total,
        forms_by_reason,
        semantic_only_obligations,
        unexcluded_pages: check.unexcluded_pages.len(),
        problems: check.problems.len(),
        families,
    }
}
