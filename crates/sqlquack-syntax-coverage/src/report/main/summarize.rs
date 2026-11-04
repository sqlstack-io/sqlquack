//! `summarize` and `by_page`: counts of a check, by family or by documentation page.

use std::collections::BTreeMap;

use crate::check::models::{CheckResult, FormResult, FormState};
use crate::report::models::{FamilySummary, Summary, Tally};

/// Percent with two decimals; zero of zero is zero.
fn percent(part: usize, whole: usize) -> f64 {
    match whole {
        0 => 0.0,
        _ => (part as f64 * 10_000.0 / whole as f64).round() / 100.0,
    }
}

fn add(tally: &mut Tally, form: &FormResult) {
    tally.forms += 1;
    tally.verified += usize::from(form.state == FormState::Verified);
    tally.excluded += usize::from(form.state == FormState::Excluded);
    tally.required_obligations += form.required_obligations;
    tally.satisfied_obligations += form.satisfied_obligations;
}

/// Tallies of the counted forms, keyed by `key`.
pub fn tally_by(check: &CheckResult, key: fn(&FormResult) -> &str) -> BTreeMap<String, Tally> {
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for form in check
        .forms
        .iter()
        .filter(|form| form.state != FormState::Absent)
    {
        add(tallies.entry(key(form).to_owned()).or_default(), form);
    }
    tallies
}

pub fn summarize(check: &CheckResult) -> Summary {
    let mut total: Tally = Tally::default();
    let mut forms_by_reason = BTreeMap::new();
    let mut semantic_only_obligations: usize = 0;
    for form in check
        .forms
        .iter()
        .filter(|form| form.state != FormState::Absent)
    {
        add(&mut total, form);
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
