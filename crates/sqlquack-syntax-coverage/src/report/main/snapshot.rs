//! `snapshot`: the claims of one check, bound to the catalogue it ran on.

use std::collections::BTreeSet;

use crate::check::models::{CheckResult, LocalVerdict, SnowflakeSyntax};
use crate::formats::constants::{ALIASES_FILE, CATALOGUE_DIR, SNAPSHOT_FORMAT, SOURCES_FILE};
use crate::formats::models::SyntaxCoverage;
use crate::report::models::Snapshot;

/// Whether a coverage path belongs to the catalogue rather than to obligations or evidence.
fn is_catalogue_file(path: &str) -> bool {
    path == SOURCES_FILE || path == ALIASES_FILE || path.starts_with(&format!("{CATALOGUE_DIR}/"))
}

/// Cases whose receipts say `snowflake` and whose parser verdict is acceptance when `accepted`.
fn by_snowflake(
    check: &CheckResult,
    snowflake: SnowflakeSyntax,
    accepted: bool,
) -> BTreeSet<String> {
    check
        .forms
        .iter()
        .flat_map(|form| &form.obligations)
        .flat_map(|obligation| &obligation.cases)
        .filter(|case| {
            case.snowflake == Some(snowflake)
                && matches!(case.local, LocalVerdict::Accepted { .. }) == accepted
                && !matches!(case.local, LocalVerdict::Failed { .. })
        })
        .map(|case| case.case_id.clone())
        .collect()
}

/// The snapshot of `check` on `coverage`, the tree it checked.
pub fn snapshot(
    check: &CheckResult,
    coverage: &SyntaxCoverage,
    catalogue_commit: &str,
) -> Snapshot {
    Snapshot {
        format: SNAPSHOT_FORMAT.to_owned(),
        catalogue_commit: catalogue_commit.to_owned(),
        catalogue_files: coverage
            .file_hashes
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
        parsed_cases: by_snowflake(check, SnowflakeSyntax::Parsed, true),
        refused_cases: by_snowflake(check, SnowflakeSyntax::Parsed, false),
        syntax_error_cases_rejected: by_snowflake(check, SnowflakeSyntax::Refused, false),
        syntax_error_cases_accepted: by_snowflake(check, SnowflakeSyntax::Refused, true),
        documented_cases: coverage.documented_cases.clone(),
    }
}
