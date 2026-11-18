//! `screen_cases`: every accepted case's same-tree variants, held against the allowlist.

use std::collections::BTreeSet;

use crate::screen::_helpers::mutations::mutations;
use crate::screen::models::{
    AllowedSameTree, MutationKind, SameTree, ScreenAllowlist, ScreenReport, ScreenedCase,
};
use crate::screen::types::{TokensOf, TreeOf};

fn allows(entry: &AllowedSameTree, found: &SameTree) -> bool {
    entry.form == found.form
        && entry.mutation == found.mutation.kind
        && entry
            .token
            .as_ref()
            .is_none_or(|token| *token == found.mutation.token)
        && (!entry.last_token || found.mutation.last)
        && (!entry.untokenized || found.mutation.untokenized)
}

/// Entries pinned to neither a token nor untokenized text, and lexical entries other than requote.
fn unpinned(allowlist: &ScreenAllowlist) -> Vec<String> {
    let entries = allowlist
        .allowed
        .iter()
        .filter(|entry| entry.token.is_none() && !entry.untokenized)
        .map(|entry| {
            format!(
                "{} {:?}: names neither a token nor untokenized text",
                entry.form, entry.mutation
            )
        });
    let lexical = allowlist
        .lexical
        .iter()
        .filter(|entry| entry.mutation != MutationKind::Requote)
        .map(|entry| {
            format!(
                "lexical {:?}: only requote reads as the same token",
                entry.mutation
            )
        });
    entries.chain(lexical).collect()
}

/// Every accepted case's same-tree variants; on a `full` run any unused entry is stale.
pub fn screen_cases(
    cases: &[ScreenedCase],
    (tree, lexed): (&TreeOf, &TokensOf),
    allowlist: &ScreenAllowlist,
    full: bool,
) -> ScreenReport {
    let mut report: ScreenReport = ScreenReport {
        unpinned: unpinned(allowlist),
        ..ScreenReport::default()
    };
    let mut used: BTreeSet<usize> = BTreeSet::new();
    let screened: BTreeSet<&str> = cases.iter().map(|case| case.form.as_str()).collect();
    for case in cases {
        let Some(original) = tree(&case.sql) else {
            continue;
        };
        report.cases += 1;
        for mutation in mutations(&case.sql, &lexed(&case.sql)) {
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
            if let Some(entry) = allowlist.lexical.iter().find(|entry| {
                entry.mutation == MutationKind::Requote && entry.mutation == found.mutation.kind
            }) {
                report.allowed.push((found, entry.reason.clone()));
                continue;
            }
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
        .filter(|(index, entry)| {
            (full || screened.contains(entry.form.as_str())) && !used.contains(index)
        })
        .map(|(_, entry)| match screened.contains(entry.form.as_str()) {
            true => format!(
                "{} {:?}: no same-tree variant is left",
                entry.form, entry.mutation
            ),
            false => format!(
                "{} {:?}: no case of this form is screened",
                entry.form, entry.mutation
            ),
        })
        .collect();
    report
}
