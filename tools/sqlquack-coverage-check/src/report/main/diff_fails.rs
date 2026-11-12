//! `diff_fails`: whether a diff fails the gate.

use crate::report::models::SnapshotDiff;

/// A regression, an unlisted receipt disagreement, a stale or added entry, or a catalogue change.
pub fn diff_fails(diff: &SnapshotDiff) -> bool {
    [
        &diff.unlisted_syntax_errors_accepted,
        &diff.unlisted_parsed_cases_refused,
        &diff.unlisted_receipts_contradicted,
        &diff.disagreements_resolved,
        &diff.disagreements_added,
        &diff.forms_lost,
        &diff.obligations_lost,
        &diff.receipts_contradicted,
        &diff.parsed_cases_refused,
        &diff.syntax_error_cases_accepted,
        &diff.catalogue_files_changed,
        &diff.forms_added,
        &diff.forms_removed,
    ]
    .iter()
    .any(|ids| !ids.is_empty())
}
