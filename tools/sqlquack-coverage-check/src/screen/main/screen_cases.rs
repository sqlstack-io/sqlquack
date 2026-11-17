//! `screen_cases`: every accepted case's same-tree variants, held against the allowlist.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::screen::_helpers::mutations::mutations;
use crate::screen::models::{
    AllowedSameTree, SameTree, ScreenAllowlist, ScreenReport, ScreenedCase,
};

fn allows(entry: &AllowedSameTree, found: &SameTree) -> bool {
    entry.form == found.form
        && entry.mutation == found.mutation.kind
        && entry
            .token
            .as_ref()
            .is_none_or(|token| *token == found.mutation.token)
        && (!entry.last_token || found.mutation.last)
}

/// Parses each case and its variants with `tree`, which gives none for a refused text.
pub fn screen_cases(
    cases: &[ScreenedCase],
    tree: &dyn Fn(&str) -> Option<Value>,
    allowlist: &ScreenAllowlist,
) -> ScreenReport {
    let mut report: ScreenReport = ScreenReport::default();
    let mut used: BTreeSet<usize> = BTreeSet::new();
    let screened: BTreeSet<&str> = cases.iter().map(|case| case.form.as_str()).collect();
    for case in cases {
        let Some(original) = tree(&case.sql) else {
            continue;
        };
        report.cases += 1;
        for mutation in mutations(&case.sql) {
            report.variants += 1;
            if tree(&mutation.sql).as_ref() != Some(&original) {
                continue;
            }
            let found: SameTree = SameTree {
                form: case.form.clone(),
                case_id: case.case_id.clone(),
                mutation,
                original: case.sql.clone(),
            };
            match allowlist
                .allowed
                .iter()
                .position(|entry| allows(entry, &found))
            {
                Some(index) => {
                    used.insert(index);
                    let reason: String = allowlist.allowed[index].reason.clone();
                    report.allowed.push((found, reason));
                }
                None => report.flagged.push(found),
            }
        }
    }
    report.stale = allowlist
        .allowed
        .iter()
        .enumerate()
        .filter(|(index, entry)| screened.contains(entry.form.as_str()) && !used.contains(index))
        .map(|(_, entry)| {
            format!(
                "{} {:?}: no same-tree variant is left",
                entry.form, entry.mutation
            )
        })
        .collect();
    report
}
