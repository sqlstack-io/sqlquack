//! `snapshot`: the claims of one check, bound to the catalogue it ran on.

use std::collections::BTreeMap;

use crate::check::models::CheckResult;
use crate::formats::constants::{ALIASES_FILE, CATALOGUE_DIR, SNAPSHOT_FORMAT, SOURCES_FILE};
use crate::report::models::Snapshot;

/// Whether a coverage path belongs to the catalogue rather than to obligations or evidence.
fn is_catalogue_file(path: &str) -> bool {
    path == SOURCES_FILE || path == ALIASES_FILE || path.starts_with(&format!("{CATALOGUE_DIR}/"))
}

/// The snapshot of `check`; `file_hashes` are the loaded files' hashes by relative path.
pub fn snapshot(
    check: &CheckResult,
    file_hashes: &BTreeMap<String, String>,
    catalogue_commit: &str,
) -> Snapshot {
    Snapshot {
        format: SNAPSHOT_FORMAT.to_owned(),
        catalogue_commit: catalogue_commit.to_owned(),
        catalogue_files: file_hashes
            .iter()
            .filter(|(path, _)| is_catalogue_file(path))
            .map(|(path, hash)| (path.clone(), hash.clone()))
            .collect(),
        parser_tree: check.parser_tree.clone(),
        forms: check
            .forms
            .iter()
            .map(|form| (form.id.clone(), form.state))
            .collect(),
        satisfied_obligations: check
            .forms
            .iter()
            .flat_map(|form| &form.obligations)
            .filter(|obligation| obligation.required && obligation.reason.is_none())
            .map(|obligation| obligation.id.clone())
            .collect(),
    }
}
