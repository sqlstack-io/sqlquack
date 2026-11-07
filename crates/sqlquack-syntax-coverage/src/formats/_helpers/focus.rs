//! The checks of a focus file: its header and each entry.

use std::collections::{BTreeMap, BTreeSet};

use crate::formats::_helpers::dialect::current_dialect;
use crate::formats::constants::{FOCUS_USES, GIT_COMMIT_LENGTH};
use crate::formats::models::{FocusEntry, FocusFile, FormStatus};

/// A full lower-case Git commit id.
fn is_commit(text: &str) -> bool {
    text.len() == GIT_COMMIT_LENGTH
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Sorted distinct letters from A to H, at least one.
fn valid_uses(uses: &[String]) -> bool {
    !uses.is_empty()
        && uses.windows(2).all(|pair| pair[0] < pair[1])
        && uses
            .iter()
            .all(|letter| FOCUS_USES.contains(&letter.as_str()))
}

/// The header's problems: another dialect, a short commit or no entries.
pub(crate) fn header_problems(focus: &FocusFile, place: &str) -> Vec<String> {
    let dialect: String = current_dialect().name.clone();
    let faults: [(bool, String); 3] = [
        (
            focus.dialect != dialect,
            format!("dialect {} is not the root's {dialect}", focus.dialect),
        ),
        (
            !is_commit(&focus.sqlbuild_commit),
            "sqlbuild_commit is not a full lower-case Git commit id".to_owned(),
        ),
        (focus.entries.is_empty(), "lists no focus form".to_owned()),
    ];
    faults
        .into_iter()
        .filter(|(fault, _)| *fault)
        .map(|(_, problem)| format!("{place}: {problem}"))
        .collect()
}

/// One entry's problems against the catalogue's form statuses and the forms seen before it.
pub(crate) fn entry_problems(
    entry: &FocusEntry,
    statuses: &BTreeMap<&str, FormStatus>,
    seen: &BTreeSet<&str>,
    place: &str,
) -> Vec<String> {
    let form: &str = &entry.form;
    let status: Option<&FormStatus> = statuses.get(form);
    let faults: [(bool, &str); 5] = [
        (status.is_none(), "is not a catalogued form"),
        (
            status == Some(&FormStatus::AbsentFromSource),
            "is absent from its source; focus the form that carries it",
        ),
        (seen.contains(form), "is listed twice"),
        (
            !valid_uses(&entry.uses),
            "uses are not sorted distinct letters from A to H",
        ),
        (
            entry
                .note
                .as_deref()
                .is_some_and(|note| note.trim().is_empty()),
            "has a blank note",
        ),
    ];
    faults
        .into_iter()
        .filter(|(fault, _)| *fault)
        .map(|(_, problem)| format!("{place}: {form} {problem}"))
        .collect()
}
