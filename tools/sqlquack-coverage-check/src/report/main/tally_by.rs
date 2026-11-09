//! `tally_by`: counts of a check keyed by family, page or any other form field.

use std::collections::BTreeMap;

use crate::check::models::{CheckResult, FormResult, FormState};
use crate::report::_helpers::tally::counted;
use crate::report::models::Tally;

/// Tallies of the counted forms, keyed by `key`; absent forms are never counted.
pub fn tally_by(check: &CheckResult, key: fn(&FormResult) -> &str) -> BTreeMap<String, Tally> {
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for form in check
        .forms
        .iter()
        .filter(|form| form.state != FormState::Absent)
    {
        let tally: Tally = tallies.remove(key(form)).unwrap_or_default();
        tallies.insert(key(form).to_owned(), counted(tally, form));
    }
    tallies
}
