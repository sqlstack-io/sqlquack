//! `focus_report`: how far the focus forms are, in work order and by SQLBuild use.

use std::collections::BTreeMap;

use crate::check::models::{CheckResult, FormResult, FormState};
use crate::formats::models::FocusFile;
use crate::report::models::{FocusLine, FocusReport, FocusTally};

/// `tally` with `form` counted in it.
fn added(tally: FocusTally, form: &FormResult) -> FocusTally {
    FocusTally {
        forms: tally.forms + 1,
        verified: tally.verified + usize::from(form.state == FormState::Verified),
        required_obligations: tally.required_obligations + form.required_obligations,
        satisfied_obligations: tally.satisfied_obligations + form.satisfied_obligations,
    }
}

/// The focus file's forms against `check`; `load_focus` has already checked every id.
pub fn focus_report(focus: &FocusFile, check: &CheckResult) -> FocusReport {
    let results: BTreeMap<&str, &FormResult> = check
        .forms
        .iter()
        .map(|form| (form.id.as_str(), form))
        .collect();
    let mut total: FocusTally = FocusTally::default();
    let mut by_use: BTreeMap<String, FocusTally> = BTreeMap::new();
    let mut forms: Vec<FocusLine> = Vec::new();
    for entry in &focus.entries {
        let Some(form) = results.get(entry.form.as_str()) else {
            continue;
        };
        total = added(total, form);
        for letter in &entry.uses {
            let tally: FocusTally = by_use.remove(letter).unwrap_or_default();
            by_use.insert(letter.clone(), added(tally, form));
        }
        forms.push(FocusLine {
            form: entry.form.clone(),
            uses: entry.uses.clone(),
            state: form.state,
            required_obligations: form.required_obligations,
            satisfied_obligations: form.satisfied_obligations,
            reasons: form.reasons.clone(),
        });
    }
    FocusReport {
        dialect: focus.dialect.clone(),
        sqlbuild_commit: focus.sqlbuild_commit.clone(),
        total,
        by_use,
        forms,
    }
}
