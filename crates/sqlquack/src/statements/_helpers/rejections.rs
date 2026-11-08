//! Keeps a matching rejection only where SQLQuack's refusal is specific to the invalid use.

use std::collections::{BTreeMap, BTreeSet};

use sqlquack_core::diagnostics::constants::MISSING_RULE;
use sqlquack_evidence::cases::models::LoadedCase;

use crate::statements::models::{Outcome, Statement};

/// The references of the cases that declare `compile_error`.
pub(crate) fn declared_compile_errors<'a>(
    cases: &BTreeMap<&'a str, &LoadedCase>,
) -> BTreeSet<&'a str> {
    cases
        .iter()
        .filter(|(_, case)| case.compile_error)
        .map(|(reference, _)| *reference)
        .collect()
}

/// `statements` with each unspecific matching rejection turned back into a refusal.
pub(crate) fn keep_specific_rejections(
    mut statements: Vec<Statement>,
    declared: &BTreeSet<&str>,
) -> Vec<Statement> {
    let refused_valid = refused_valid_keys(&statements, declared);
    for statement in &mut statements {
        if statement.outcome == Outcome::MatchingRejection
            && !is_specific(statement, &refused_valid)
        {
            statement.outcome = Outcome::Refused;
        }
    }
    statements
}

/// The refused construct keys of each case file's refused cases that are not in `declared`.
fn refused_valid_keys(
    statements: &[Statement],
    declared: &BTreeSet<&str>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut keys: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for statement in statements {
        if statement.outcome == Outcome::Refused && !declared.contains(statement.reference.as_str())
        {
            let file = keys
                .entry(case_file(&statement.reference).to_owned())
                .or_default();
            file.extend(statement.refused.iter().cloned());
        }
    }
    keys
}

/// Whether `statement`'s refusal is neither SQ0002 nor on a key a valid case of its file is refused on.
fn is_specific(statement: &Statement, refused_valid: &BTreeMap<String, BTreeSet<String>>) -> bool {
    let empty: BTreeSet<String> = BTreeSet::new();
    let valid = refused_valid
        .get(case_file(&statement.reference))
        .unwrap_or(&empty);
    !statement.codes.iter().any(|code| code == MISSING_RULE)
        && !statement.refused.iter().any(|key| valid.contains(key))
}

/// The case file of a `<file>#<id>` reference.
fn case_file(reference: &str) -> &str {
    reference
        .split_once('#')
        .map_or(reference, |(file, _)| file)
}
